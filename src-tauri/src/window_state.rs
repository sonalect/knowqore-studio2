//! Persists the main window's position and size, and keep it on a visible monitor.
//!
//! A saved placement is restored only after it is fitted to a current work area.
//! If the second display was unplugged, the window is pulled onto a remaining screen
//! instead of opening in the old, now invisible, coordinates.
//!
//! The mode (maximized or not) and the normal bounds are kept apart: a window
//! closed maximized reopens maximized, and Restore then returns it to the size
//! it had before it was maximized, not to the size of the display. Windows
//! keeps the normal bounds itself (`WINDOWPLACEMENT.rcNormalPosition`); on
//! macOS and Linux they are remembered here while the window is not maximized.

#![cfg_attr(windows, allow(unsafe_code))]

use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::sync::PoisonError;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

const STATE_FILE: &str = "window-state.json";
const MIN_LOGICAL_WIDTH: f64 = 330.0;
const MIN_LOGICAL_HEIGHT: f64 = 120.0;
const TITLEBAR_LOGICAL: f64 = 32.0;
const MIN_VISIBLE_TITLE_LOGICAL: f64 = 80.0;
const DEFAULT_LOGICAL_WIDTH: f64 = 800.0;
const DEFAULT_LOGICAL_HEIGHT: f64 = 600.0;
const FULLSCREEN_SLACK: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SavedWindowState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

/// The last bounds of the window while it was not maximized, shared by the
/// event handler. `None` until the window has reported or restored them.
type NormalBounds = Arc<Mutex<Option<Rect>>>;

impl SavedWindowState {
    fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

/// Restores the saved placement of `window` onto a visible display, then
/// saves it on every move, resize, and close. Failures are logged, never fatal.
pub fn attach(window: &WebviewWindow) {
    let restored = restore(window).unwrap_or_else(|error| {
        eprintln!("failed to restore window state: {error}");
        None
    });
    let normal: NormalBounds = Arc::new(Mutex::new(restored));

    let watched = window.clone();
    window.on_window_event(move |event| {
        let should_rescue = matches!(
            event,
            WindowEvent::Moved(_)
                | WindowEvent::Resized(_)
                | WindowEvent::Focused(true)
                | WindowEvent::ScaleFactorChanged { .. }
        );
        let should_save = should_rescue || matches!(event, WindowEvent::CloseRequested { .. });
        if should_rescue && let Err(error) = rescue_if_needed(&watched) {
            eprintln!("failed to keep the window on screen: {error}");
        }
        if should_save && let Err(error) = persist(&watched, &normal) {
            eprintln!("failed to save window state: {error}");
        }
    });
}

/// Applies the saved placement and returns the normal bounds it restored, or
/// `None` when there was nothing to restore.
fn restore(window: &WebviewWindow) -> Result<Option<Rect>, String> {
    let Some(saved) = load_state(window)? else {
        return Ok(None);
    };
    if saved.width == 0 || saved.height == 0 {
        return Ok(None);
    }

    let work_areas = collect_work_areas(window);
    let (min_width, min_height) = min_physical_size(window);
    let scale = window.scale_factor().unwrap_or(1.0);
    let default_size = (
        logical_to_physical(DEFAULT_LOGICAL_WIDTH, scale),
        logical_to_physical(DEFAULT_LOGICAL_HEIGHT, scale),
    );
    let normal = replacement_if_saved_as_fullscreen(
        saved.rect(),
        saved.maximized,
        &work_areas,
        default_size,
    );
    let placed = restore_rect(normal, &work_areas, min_width, min_height);

    #[cfg(windows)]
    {
        apply_placement(window, placed, saved.maximized)?;
    }
    #[cfg(not(windows))]
    {
        // The size set before `maximize` is the one Restore returns to.
        window
            .set_size(PhysicalSize {
                width: placed.width,
                height: placed.height,
            })
            .map_err(|error| error.to_string())?;
        window
            .set_position(PhysicalPosition {
                x: placed.x,
                y: placed.y,
            })
            .map_err(|error| error.to_string())?;
        if saved.maximized {
            window.maximize().map_err(|error| error.to_string())?;
        }
    }
    Ok(Some(placed))
}

fn persist(window: &WebviewWindow, normal_bounds: &NormalBounds) -> Result<(), String> {
    if window.is_minimized().unwrap_or(false) {
        return Ok(());
    }

    let (normal, maximized) = snapshot(window, normal_bounds)?;
    if normal.width == 0 || normal.height == 0 {
        return Ok(());
    }

    save_state(
        window,
        &SavedWindowState {
            x: normal.x,
            y: normal.y,
            width: normal.width,
            height: normal.height,
            maximized,
        },
    )
}

/// The normal bounds to save and whether the window is maximized.
fn snapshot(window: &WebviewWindow, normal_bounds: &NormalBounds) -> Result<(Rect, bool), String> {
    #[cfg(windows)]
    {
        // Windows reports the normal bounds of a maximized window itself.
        let _ = normal_bounds;
        read_placement(window)
    }
    #[cfg(not(windows))]
    {
        let current = current_rect(window)?;
        let maximized = window.is_maximized().unwrap_or(false);
        let work_areas = collect_work_areas(window);
        let mut normal = normal_bounds.lock().unwrap_or_else(PoisonError::into_inner);
        *normal = track_normal(*normal, current, maximized, &work_areas);
        Ok((saved_bounds(*normal, current, maximized), maximized))
    }
}

/// The normal bounds to remember after the window reported `current`.
///
/// A maximized window keeps the previous ones. So does a window that fills
/// its work area without being maximized yet: the resize of a maximize can
/// arrive before the window reports the new mode.
#[cfg(any(not(windows), test))]
pub(crate) fn track_normal(
    previous: Option<Rect>,
    current: Rect,
    maximized: bool,
    work_areas: &[Rect],
) -> Option<Rect> {
    if maximized || fills_work_area(current, work_areas) {
        previous
    } else {
        Some(current)
    }
}

/// The bounds to save: the remembered normal ones while maximized, else the
/// current ones.
#[cfg(any(not(windows), test))]
pub(crate) fn saved_bounds(normal: Option<Rect>, current: Rect, maximized: bool) -> Rect {
    match normal {
        Some(normal) if maximized => normal,
        _ => current,
    }
}

fn rescue_if_needed(window: &WebviewWindow) -> Result<(), String> {
    if window.is_minimized().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
        return Ok(());
    }

    let current = current_rect(window)?;
    let work_areas = collect_work_areas(window);
    let scale = window.scale_factor().unwrap_or(1.0);
    let title_height = logical_to_physical(TITLEBAR_LOGICAL, scale);
    let min_visible = logical_to_physical(MIN_VISIBLE_TITLE_LOGICAL, scale);
    if !needs_rescue(current, &work_areas, title_height, min_visible) {
        return Ok(());
    }

    let (min_width, min_height) = min_physical_size(window);
    let placed = restore_rect(current, &work_areas, min_width, min_height);
    if placed == current {
        return Ok(());
    }
    window
        .set_size(PhysicalSize {
            width: placed.width,
            height: placed.height,
        })
        .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition {
            x: placed.x,
            y: placed.y,
        })
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn current_rect(window: &WebviewWindow) -> Result<Rect, String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    Ok(Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

fn collect_work_areas(window: &WebviewWindow) -> Vec<Rect> {
    let mut areas = Vec::new();
    if let Ok(Some(primary)) = window.primary_monitor() {
        areas.push(rect_from_work_area(primary.work_area()));
    }
    if let Ok(monitors) = window.available_monitors() {
        for monitor in monitors {
            let area = rect_from_work_area(monitor.work_area());
            if !areas.contains(&area) {
                areas.push(area);
            }
        }
    }
    areas
}

fn rect_from_work_area(work_area: &tauri::PhysicalRect<i32, u32>) -> Rect {
    Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width,
        height: work_area.size.height,
    }
}

fn min_physical_size(window: &WebviewWindow) -> (u32, u32) {
    let scale = window.scale_factor().unwrap_or(1.0);
    (
        logical_to_physical(MIN_LOGICAL_WIDTH, scale),
        logical_to_physical(MIN_LOGICAL_HEIGHT, scale),
    )
}

fn logical_to_physical(value: f64, scale: f64) -> u32 {
    (value * scale).round().max(1.0) as u32
}

fn state_path(window: &WebviewWindow) -> Result<PathBuf, String> {
    let dir = window
        .app_handle()
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(dir.join(STATE_FILE))
}

fn load_state(window: &WebviewWindow) -> Result<Option<SavedWindowState>, String> {
    let path = state_path(window)?;
    if !path.exists() {
        return Ok(None);
    }
    read_state_file(&path)
}

fn read_state_file(path: &Path) -> Result<Option<SavedWindowState>, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn save_state(window: &WebviewWindow, state: &SavedWindowState) -> Result<(), String> {
    let path = state_path(window)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|error| error.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(|error| error.to_string())?;
    fs::rename(&tmp, &path).map_err(|error| error.to_string())
}

pub(crate) fn replacement_if_saved_as_fullscreen(
    saved: Rect,
    maximized: bool,
    work_areas: &[Rect],
    default_size: (u32, u32),
) -> Rect {
    if !maximized || !fills_work_area(saved, work_areas) {
        return saved;
    }
    Rect {
        x: saved.x,
        y: saved.y,
        width: default_size.0,
        height: default_size.1,
    }
}

/// Whether `rect` covers the whole work area it is on, give or take
/// `FULLSCREEN_SLACK`.
fn fills_work_area(rect: Rect, work_areas: &[Rect]) -> bool {
    pick_work_area(rect, work_areas).is_some_and(|area| {
        rect.width.saturating_add(FULLSCREEN_SLACK) >= area.width
            && rect.height.saturating_add(FULLSCREEN_SLACK) >= area.height
    })
}

#[cfg(windows)]
fn native_hwnd(window: &WebviewWindow) -> Result<windows::Win32::Foundation::HWND, String> {
    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    Ok(windows::Win32::Foundation::HWND(hwnd.0 as *mut _))
}

#[cfg(windows)]
fn read_placement(window: &WebviewWindow) -> Result<(Rect, bool), String> {
    use std::mem::size_of;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, SW_SHOWMAXIMIZED, WINDOWPLACEMENT, WPF_RESTORETOMAXIMIZED,
    };

    let hwnd = native_hwnd(window)?;
    let mut placement = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe {
        // SAFETY: `hwnd` is the live Tauri window; `length` is set as required by Win32.
        GetWindowPlacement(hwnd, &mut placement)
    }
    .map_err(|error| error.to_string())?;

    let maximized = placement.showCmd == SW_SHOWMAXIMIZED.0 as u32
        || placement.flags.contains(WPF_RESTORETOMAXIMIZED)
        || window.is_maximized().unwrap_or(false);
    Ok((rect_from_win32(placement.rcNormalPosition), maximized))
}

#[cfg(windows)]
fn apply_placement(window: &WebviewWindow, placed: Rect, maximized: bool) -> Result<(), String> {
    use std::mem::size_of;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, SW_HIDE, SW_SHOWMAXIMIZED, SetWindowPlacement, ShowWindow,
        WINDOWPLACEMENT,
    };

    let hwnd = native_hwnd(window)?;
    let mut placement = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe {
        // SAFETY: `hwnd` is the live Tauri window; `length` is set as required by Win32.
        GetWindowPlacement(hwnd, &mut placement)
    }
    .map_err(|error| error.to_string())?;

    placement.rcNormalPosition = rect_to_win32(placed);
    placement.showCmd = if maximized {
        SW_SHOWMAXIMIZED.0 as u32
    } else {
        SW_HIDE.0 as u32
    };
    unsafe {
        // SAFETY: `hwnd` is valid; this only updates restored bounds and show command.
        SetWindowPlacement(hwnd, &placement)
    }
    .map_err(|error| error.to_string())?;

    if maximized {
        // SetWindowPlacement(SW_SHOWMAXIMIZED) can make a hidden window visible.
        // Hide it again so the frontend still reveals after the first paint.
        unsafe {
            // SAFETY: hide after SW_SHOWMAXIMIZED so the first paint still happens off-screen.
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn rect_from_win32(rect: windows::Win32::Foundation::RECT) -> Rect {
    Rect {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(0) as u32,
        height: (rect.bottom - rect.top).max(0) as u32,
    }
}

#[cfg(windows)]
fn rect_to_win32(rect: Rect) -> windows::Win32::Foundation::RECT {
    windows::Win32::Foundation::RECT {
        left: rect.x,
        top: rect.y,
        right: rect.x.saturating_add(as_i32(rect.width)),
        bottom: rect.y.saturating_add(as_i32(rect.height)),
    }
}

pub(crate) fn restore_rect(
    saved: Rect,
    work_areas: &[Rect],
    min_width: u32,
    min_height: u32,
) -> Rect {
    let Some(target) = pick_work_area(saved, work_areas) else {
        return saved;
    };

    let width = fit_size(saved.width, min_width, target.width);
    let height = fit_size(saved.height, min_height, target.height);
    Rect {
        x: clamp_axis(saved.x, target.x, target.width, width),
        y: clamp_axis(saved.y, target.y, target.height, height),
        width,
        height,
    }
}

pub(crate) fn needs_rescue(
    window: Rect,
    work_areas: &[Rect],
    title_height: u32,
    min_visible_width: u32,
) -> bool {
    if work_areas.is_empty() {
        return false;
    }
    visible_title_width(window, work_areas, title_height) < min_visible_width
}

fn visible_title_width(window: Rect, work_areas: &[Rect], title_height: u32) -> u32 {
    let title = Rect {
        x: window.x,
        y: window.y,
        width: window.width,
        height: title_height.min(window.height.max(1)),
    };
    work_areas
        .iter()
        .filter_map(|area| intersection(title, *area))
        .map(|part| part.width)
        .max()
        .unwrap_or(0)
}

fn pick_work_area(saved: Rect, work_areas: &[Rect]) -> Option<Rect> {
    let mut best: Option<(u64, u64, Rect)> = None;
    for area in work_areas {
        let overlap = intersection(saved, *area).map_or(0, area_of);
        let distance = center_distance_sq(saved, *area);
        let better = match best {
            None => true,
            Some((best_overlap, best_distance, _)) => {
                overlap > best_overlap || (overlap == best_overlap && distance < best_distance)
            }
        };
        if better {
            best = Some((overlap, distance, *area));
        }
    }
    best.map(|(_, _, area)| area)
}

fn fit_size(saved: u32, min: u32, work: u32) -> u32 {
    let work = work.max(1);
    saved.max(min).min(work).max(1)
}

fn clamp_axis(value: i32, work_start: i32, work_len: u32, size: u32) -> i32 {
    let size = as_i32(size);
    let work_len = as_i32(work_len);
    let max_start = work_start.saturating_add(work_len.saturating_sub(size));
    if max_start < work_start {
        work_start
    } else {
        value.clamp(work_start, max_start)
    }
}

fn intersection(a: Rect, b: Rect) -> Option<Rect> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    let width = right.checked_sub(x)?;
    let height = bottom.checked_sub(y)?;
    if width <= 0 || height <= 0 {
        return None;
    }
    Some(Rect {
        x,
        y,
        width: width as u32,
        height: height as u32,
    })
}

fn area_of(rect: Rect) -> u64 {
    u64::from(rect.width) * u64::from(rect.height)
}

fn center_distance_sq(a: Rect, b: Rect) -> u64 {
    let (ax, ay) = center(a);
    let (bx, by) = center(b);
    let dx = ax.abs_diff(bx);
    let dy = ay.abs_diff(by);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn center(rect: Rect) -> (i64, i64) {
    let x = i64::from(rect.x) + i64::from(as_i32(rect.width)) / 2;
    let y = i64::from(rect.y) + i64::from(as_i32(rect.height)) / 2;
    (x, y)
}

impl Rect {
    fn right(self) -> i32 {
        self.x.saturating_add(as_i32(self.width))
    }

    fn bottom(self) -> i32 {
        self.y.saturating_add(as_i32(self.height))
    }
}

fn as_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        Rect, needs_rescue, replacement_if_saved_as_fullscreen, restore_rect, saved_bounds,
        track_normal,
    };

    const PRIMARY: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    };
    const SECOND: Rect = Rect {
        x: 1920,
        y: 0,
        width: 1920,
        height: 1080,
    };

    #[test]
    fn keeps_a_window_that_already_fits_the_primary() {
        let saved = Rect {
            x: 120,
            y: 80,
            width: 800,
            height: 600,
        };
        assert_eq!(restore_rect(saved, &[PRIMARY], 330, 120), saved);
    }

    #[test]
    fn pulls_a_window_back_when_the_right_monitor_is_gone() {
        let saved = Rect {
            x: 2200,
            y: 80,
            width: 800,
            height: 600,
        };
        let placed = restore_rect(saved, &[PRIMARY], 330, 120);
        assert_eq!(placed.width, 800);
        assert_eq!(placed.height, 600);
        assert!(placed.x >= PRIMARY.x);
        assert!(
            i64::from(placed.x) + i64::from(placed.width)
                <= i64::from(PRIMARY.x) + i64::from(PRIMARY.width)
        );
        assert!(placed.y >= PRIMARY.y);
        assert!(
            i64::from(placed.y) + i64::from(placed.height)
                <= i64::from(PRIMARY.y) + i64::from(PRIMARY.height)
        );
    }

    #[test]
    fn pulls_a_window_back_when_the_left_monitor_is_gone() {
        let saved = Rect {
            x: -1600,
            y: 40,
            width: 800,
            height: 600,
        };
        let placed = restore_rect(saved, &[PRIMARY], 330, 120);
        assert_eq!(
            placed,
            Rect {
                x: 0,
                y: 40,
                width: 800,
                height: 600,
            }
        );
    }

    #[test]
    fn prefers_the_monitor_with_the_largest_overlap() {
        let saved = Rect {
            x: 2000,
            y: 100,
            width: 800,
            height: 600,
        };
        let placed = restore_rect(saved, &[PRIMARY, SECOND], 330, 120);
        assert!(placed.x >= SECOND.x);
        assert!(
            i64::from(placed.x) + i64::from(placed.width)
                <= i64::from(SECOND.x) + i64::from(SECOND.width)
        );
    }

    #[test]
    fn shrinks_to_the_work_area_when_the_saved_size_is_too_large() {
        let saved = Rect {
            x: 0,
            y: 0,
            width: 3000,
            height: 2000,
        };
        let placed = restore_rect(saved, &[PRIMARY], 330, 120);
        assert_eq!(placed, PRIMARY);
    }

    #[test]
    fn stays_put_when_no_monitors_are_reported() {
        let saved = Rect {
            x: 4000,
            y: 4000,
            width: 800,
            height: 600,
        };
        assert_eq!(restore_rect(saved, &[], 330, 120), saved);
    }

    #[test]
    fn rescues_a_window_whose_title_bar_is_off_screen() {
        let window = Rect {
            x: 4000,
            y: 0,
            width: 800,
            height: 600,
        };
        assert!(needs_rescue(window, &[PRIMARY], 32, 80));
    }

    #[test]
    fn leaves_a_window_that_still_has_a_grabbable_title_bar() {
        let window = Rect {
            x: 1840,
            y: 0,
            width: 800,
            height: 600,
        };
        assert!(!needs_rescue(window, &[PRIMARY], 32, 80));
    }

    #[test]
    fn chooses_the_nearest_remaining_display_for_a_left_monitor() {
        let saved = Rect {
            x: -1920,
            y: 0,
            width: 800,
            height: 600,
        };
        let placed = restore_rect(saved, &[PRIMARY, SECOND], 330, 120);
        assert!(placed.x >= PRIMARY.x);
        assert!(placed.x < PRIMARY.x + 1920);
        assert_eq!(placed.width, 800);
    }

    #[test]
    fn does_not_place_the_window_under_the_taskbar() {
        let saved = Rect {
            x: 100,
            y: 900,
            width: 800,
            height: 600,
        };
        let placed = restore_rect(saved, &[PRIMARY], 330, 120);
        assert_eq!(placed.y, 440);
        assert_eq!(placed.height, 600);
    }

    #[test]
    fn replaces_a_maximized_save_that_stored_the_full_work_area() {
        let saved = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let replaced = replacement_if_saved_as_fullscreen(saved, true, &[PRIMARY], (800, 600));
        assert_eq!(
            replaced,
            Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            }
        );
    }

    #[test]
    fn keeps_a_normal_maximized_restore_size() {
        let saved = Rect {
            x: 120,
            y: 80,
            width: 800,
            height: 600,
        };
        assert_eq!(
            replacement_if_saved_as_fullscreen(saved, true, &[PRIMARY], (800, 600)),
            saved
        );
    }

    const NORMAL: Rect = Rect {
        x: 120,
        y: 80,
        width: 800,
        height: 600,
    };

    #[test]
    fn a_maximized_window_keeps_its_normal_bounds() {
        let normal = track_normal(Some(NORMAL), PRIMARY, true, &[PRIMARY]);
        assert_eq!(normal, Some(NORMAL));
        assert_eq!(saved_bounds(normal, PRIMARY, true), NORMAL);
    }

    #[test]
    fn a_normal_window_saves_its_current_bounds() {
        let moved = Rect { x: 300, ..NORMAL };
        let normal = track_normal(Some(NORMAL), moved, false, &[PRIMARY]);
        assert_eq!(normal, Some(moved));
        assert_eq!(saved_bounds(normal, moved, false), moved);
    }

    #[test]
    fn the_resize_of_a_maximize_does_not_replace_the_normal_bounds() {
        // The window already fills the display but does not report maximized yet.
        assert_eq!(
            track_normal(Some(NORMAL), PRIMARY, false, &[PRIMARY]),
            Some(NORMAL)
        );
    }

    #[test]
    fn maximize_close_reopen_keeps_the_size_restore_returns_to() {
        // Restored from the file, then maximized, then closed.
        let mut normal = Some(NORMAL);
        normal = track_normal(normal, NORMAL, false, &[PRIMARY]);
        normal = track_normal(normal, PRIMARY, false, &[PRIMARY]);
        normal = track_normal(normal, PRIMARY, true, &[PRIMARY]);
        assert_eq!(saved_bounds(normal, PRIMARY, true), NORMAL);
    }

    #[test]
    fn a_window_maximized_before_reporting_bounds_saves_the_current_ones() {
        assert_eq!(saved_bounds(None, PRIMARY, true), PRIMARY);
    }
}

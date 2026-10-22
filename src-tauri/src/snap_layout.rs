//! Native Windows 11 Snap Layout overlay for the custom maximize button.
//!
//! WebView2 owns hit-testing over the client area, so the parent window never sees
//! `WM_NCHITTEST`. A transparent child HWND over the maximize button returns
//! `HTMAXBUTTON`, which is what the shell needs to show Snap Layouts.
//!
//! This is the only module allowed to use `unsafe`.

#![allow(unsafe_code)]

use std::mem::size_of;
use std::sync::Once;

use tauri::{Emitter, Manager, WebviewWindow, WindowEvent};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, HBRUSH, NULL_BRUSH, ValidateRect};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetClientRect,
    GetWindowLongPtrW, HTMAXBUTTON, HWND_TOP, IDC_ARROW, IsZoomed, LoadCursorW, RegisterClassW,
    SC_MAXIMIZE, SC_RESTORE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SendMessageW, SetWindowLongPtrW,
    SetWindowPos, WM_DESTROY, WM_ERASEBKGND, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCMOUSELEAVE,
    WM_NCMOUSEMOVE, WM_PAINT, WM_SYSCOMMAND, WNDCLASSW, WS_CHILD, WS_CLIPSIBLINGS,
};
use windows::core::w;

const TITLEBAR_HEIGHT: i32 = 32;
const BUTTON_WIDTH: i32 = 46;
const CLOSE_BUTTONS_TO_THE_RIGHT: i32 = 1;
const HOVER_EVENT: &str = "snap-layout-hover";

struct OverlayState {
    app: tauri::AppHandle,
    parent: HWND,
    hovering: bool,
}

/// Puts the Snap Layout overlay over the maximize button of `window` and keeps
/// it there as the window resizes. Emits `snap-layout-hover` while hovered.
pub fn attach(window: &WebviewWindow) -> Result<(), String> {
    let parent = native_hwnd(window)?;
    let overlay = create_overlay(parent, window.app_handle().clone())?;
    reposition(overlay, parent)?;

    let overlay_bits = overlay.0 as isize;
    let parent_bits = parent.0 as isize;
    window.on_window_event(move |event| {
        if matches!(
            event,
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
        ) {
            let overlay = HWND(overlay_bits as *mut _);
            let parent = HWND(parent_bits as *mut _);
            let _ = reposition(overlay, parent);
        }
    });
    Ok(())
}

pub(crate) fn overlay_rect(client_width: i32, dpi: u32) -> (i32, i32, i32, i32) {
    let width = scale_px(BUTTON_WIDTH, dpi);
    let height = scale_px(TITLEBAR_HEIGHT, dpi);
    let x = client_width - width * (CLOSE_BUTTONS_TO_THE_RIGHT + 1);
    (x, 0, width, height)
}

fn scale_px(logical: i32, dpi: u32) -> i32 {
    ((f64::from(logical) * f64::from(dpi)) / 96.0).round() as i32
}

fn native_hwnd(window: &WebviewWindow) -> Result<HWND, String> {
    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    Ok(HWND(hwnd.0 as *mut _))
}

fn create_overlay(parent: HWND, app: tauri::AppHandle) -> Result<HWND, String> {
    register_class()?;

    let module = unsafe {
        // SAFETY: `None` asks for the current process module handle.
        GetModuleHandleW(None)
    }
    .map_err(|error| error.to_string())?;

    let overlay = unsafe {
        // SAFETY: the class is registered and `parent` is the live Tauri window.
        // The child is created hidden until user data is stored.
        CreateWindowExW(
            Default::default(),
            w!("TauriSnapMaxButton"),
            w!(""),
            WS_CHILD | WS_CLIPSIBLINGS,
            0,
            0,
            0,
            0,
            Some(parent),
            None,
            Some(HINSTANCE(module.0)),
            None,
        )
    }
    .map_err(|error| error.to_string())?;

    let state = Box::new(OverlayState {
        app,
        parent,
        hovering: false,
    });
    unsafe {
        // SAFETY: `overlay` is the window created above; the box is freed in `WM_DESTROY`.
        SetWindowLongPtrW(overlay, GWLP_USERDATA, Box::into_raw(state) as isize);
    }
    Ok(overlay)
}

fn register_class() -> Result<(), String> {
    static REGISTER: Once = Once::new();
    let mut error = None;
    REGISTER.call_once(|| {
        let module = match unsafe { GetModuleHandleW(None) } {
            Ok(module) => module,
            Err(err) => {
                error = Some(err.to_string());
                return;
            }
        };
        let cursor = match unsafe { LoadCursorW(None, IDC_ARROW) } {
            Ok(cursor) => cursor,
            Err(err) => {
                error = Some(err.to_string());
                return;
            }
        };
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: HINSTANCE(module.0),
            hCursor: cursor,
            hbrBackground: HBRUSH(unsafe { GetStockObject(NULL_BRUSH) }.0),
            lpszClassName: w!("TauriSnapMaxButton"),
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            error = Some("RegisterClassW failed".into());
        }
    });
    match error {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

fn reposition(overlay: HWND, parent: HWND) -> Result<(), String> {
    let mut client = RECT::default();
    unsafe { GetClientRect(parent, &mut client) }.map_err(|error| error.to_string())?;
    let dpi = unsafe { GetDpiForWindow(parent) };
    let (x, y, width, height) = overlay_rect(client.right, dpi);
    unsafe {
        // SAFETY: both handles are valid; this only moves and shows the overlay.
        SetWindowPos(
            overlay,
            Some(HWND_TOP),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    }
    .map_err(|error| error.to_string())
}

fn overlay_state(hwnd: HWND) -> Option<&'static mut OverlayState> {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut OverlayState;
    if raw.is_null() {
        None
    } else {
        // SAFETY: the pointer was stored by `create_overlay` and is cleared in `WM_DESTROY`.
        Some(unsafe { &mut *raw })
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCHITTEST => LRESULT(HTMAXBUTTON as isize),
        WM_NCMOUSEMOVE => {
            if let Some(state) = overlay_state(hwnd) {
                if !state.hovering {
                    state.hovering = true;
                    let _ = state.app.emit(HOVER_EVENT, true);
                }
                let mut track = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE | TME_NONCLIENT,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = unsafe { TrackMouseEvent(&mut track) };
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_NCMOUSELEAVE => {
            if let Some(state) = overlay_state(hwnd)
                && state.hovering
            {
                state.hovering = false;
                let _ = state.app.emit(HOVER_EVENT, false);
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_NCLBUTTONDOWN => {
            if let Some(state) = overlay_state(hwnd) {
                let command = if unsafe { IsZoomed(state.parent) }.as_bool() {
                    SC_RESTORE
                } else {
                    SC_MAXIMIZE
                };
                unsafe {
                    let _ = SendMessageW(
                        state.parent,
                        WM_SYSCOMMAND,
                        Some(WPARAM(command as usize)),
                        None,
                    );
                }
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let _ = unsafe { ValidateRect(Some(hwnd), None) };
            LRESULT(0)
        }
        WM_DESTROY => {
            let raw = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) } as *mut OverlayState;
            if !raw.is_null() {
                drop(unsafe { Box::from_raw(raw) });
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use super::{overlay_rect, scale_px};

    #[test]
    fn scales_caption_metrics_at_150_percent() {
        assert_eq!(scale_px(46, 144), 69);
        assert_eq!(scale_px(32, 144), 48);
    }

    #[test]
    fn places_overlay_over_maximize_not_close() {
        let (x, y, width, height) = overlay_rect(800, 96);
        assert_eq!((x, y, width, height), (708, 0, 46, 32));
    }
}

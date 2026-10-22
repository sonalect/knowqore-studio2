//! The system's light or dark mode and accent color, for the Astryx theme.
//!
//! Windows reports both and announces changes; on Linux and macOS the
//! command fails and the frontend follows `prefers-color-scheme` instead.

#![cfg_attr(windows, allow(unsafe_code))]

use serde::Serialize;

/// The appearance the frontend themes itself with.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAppearance {
    /// The accent color as `#rrggbb`.
    pub accent: String,
    /// Whether the system uses dark mode.
    pub dark: bool,
}

#[cfg(any(windows, test))]
fn color_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(any(windows, test))]
fn is_dark_background(r: u8, g: u8, b: u8) -> bool {
    (0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)) / 255.0 < 0.5
}

/// Tauri command `system_appearance`: the current system appearance. Fails
/// with a message outside Windows, where the frontend falls back to
/// `prefers-color-scheme`.
#[tauri::command]
pub fn system_appearance() -> Result<SystemAppearance, String> {
    #[cfg(windows)]
    {
        read_windows_appearance()
    }
    #[cfg(not(windows))]
    {
        Err("system appearance is only available on Windows".into())
    }
}

/// Emits `system-appearance-changed` whenever Windows changes its mode or
/// accent. A failure to subscribe is logged, never fatal.
#[cfg(windows)]
pub fn watch(app: &tauri::AppHandle) {
    if let Err(error) = try_watch(app) {
        eprintln!("failed to watch Windows appearance: {error}");
    }
}

/// Gives the native window the system's mode before the webview paints, so a
/// dark system never flashes a white frame.
#[cfg(windows)]
pub fn apply_window_theme(window: &tauri::WebviewWindow) {
    let appearance = match read_windows_appearance() {
        Ok(appearance) => appearance,
        Err(error) => {
            eprintln!("failed to read Windows appearance: {error}");
            return;
        }
    };
    let theme = if appearance.dark {
        tauri::Theme::Dark
    } else {
        tauri::Theme::Light
    };
    if let Err(error) = window.set_theme(Some(theme)) {
        eprintln!("failed to set window theme: {error}");
    }
    let color = if appearance.dark {
        tauri::webview::Color(0, 0, 0, 0)
    } else {
        tauri::webview::Color(255, 255, 255, 0)
    };
    if let Err(error) = window.set_background_color(Some(color)) {
        eprintln!("failed to set window background: {error}");
    }
    if let Err(error) = prepare_native_window(window, appearance.dark) {
        eprintln!("failed to prepare native window theme: {error}");
    }
}

#[cfg(windows)]
const BOOL_SIZE: u32 = std::mem::size_of::<i32>() as u32;

#[cfg(windows)]
fn prepare_native_window(window: &tauri::WebviewWindow, dark: bool) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
    use windows::Win32::Graphics::Gdi::{GetStockObject, NULL_BRUSH};
    use windows::Win32::UI::WindowsAndMessaging::{GCLP_HBRBACKGROUND, SetClassLongPtrW};

    let hwnd = HWND(window.hwnd().map_err(|error| error.to_string())?.0 as *mut _);
    let dark_mode = i32::from(dark);
    unsafe {
        // SAFETY: `hwnd` is the live Tauri window; DWM reads a BOOL-sized buffer.
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            std::ptr::from_ref(&dark_mode).cast(),
            BOOL_SIZE,
        )
    }
    .map_err(|error| error.to_string())?;

    let brush = unsafe { GetStockObject(NULL_BRUSH) };
    unsafe {
        // SAFETY: `hwnd` is valid; a null background brush stops DefWindowProc from
        // filling the client area with the default white class brush.
        SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, brush.0 as isize);
    }
    Ok(())
}

#[cfg(windows)]
fn read_windows_appearance() -> Result<SystemAppearance, String> {
    use windows::UI::ViewManagement::{UIColorType, UISettings};

    let settings = UISettings::new().map_err(|error| error.to_string())?;
    let accent = settings
        .GetColorValue(UIColorType::Accent)
        .map_err(|error| error.to_string())?;
    let background = settings
        .GetColorValue(UIColorType::Background)
        .map_err(|error| error.to_string())?;
    Ok(SystemAppearance {
        accent: color_to_hex(accent.R, accent.G, accent.B),
        dark: is_dark_background(background.R, background.G, background.B),
    })
}

#[cfg(windows)]
fn try_watch(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::{Emitter, Manager};
    use windows::Foundation::TypedEventHandler;
    use windows::UI::ViewManagement::UISettings;

    let settings = UISettings::new().map_err(|error| error.to_string())?;
    let app_handle = app.clone();
    settings
        .ColorValuesChanged(&TypedEventHandler::new(move |_, _| {
            if let Ok(appearance) = read_windows_appearance() {
                let _ = app_handle.emit("system-appearance-changed", appearance);
            }
            Ok(())
        }))
        .map_err(|error| error.to_string())?;
    app.manage(SystemAppearanceWatch {
        _settings: settings,
    });
    Ok(())
}

#[cfg(windows)]
struct SystemAppearanceWatch {
    _settings: windows::UI::ViewManagement::UISettings,
}

#[cfg(test)]
mod tests {
    use super::{color_to_hex, is_dark_background};

    #[test]
    fn formats_accent_hex() {
        assert_eq!(color_to_hex(0, 120, 212), "#0078d4");
    }

    #[test]
    fn treats_white_background_as_light() {
        assert!(!is_dark_background(255, 255, 255));
    }

    #[test]
    fn treats_black_background_as_dark() {
        assert!(is_dark_background(0, 0, 0));
    }
}

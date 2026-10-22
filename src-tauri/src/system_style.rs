//! The system's accent color and interface font on Linux and macOS, for the
//! Astryx theme. Windows reports its accent with the system appearance.
//!
//! The accent comes from the `mundy` crate (the XDG Settings portal, with a
//! Yaru fallback, on Linux; `NSColor.controlAccentColor` on macOS); the font
//! from GTK's `gtk-font-name` on Linux, while macOS already draws the page in
//! its system font. Both are read before the window shows and again whenever
//! it regains focus, since they are changed in the system settings, another
//! window; a change is sent as `system-style-changed`.

use serde::Serialize;

/// The accent color and interface font the page is themed with.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SystemStyle {
    /// The accent color as `#rrggbb`, or empty when the system reports none.
    pub accent: String,
    /// The font family of the interface, or empty to keep the theme's.
    pub font: String,
}

#[cfg(not(windows))]
use std::sync::{Mutex, PoisonError};
#[cfg(not(windows))]
use std::time::Duration;

#[cfg(not(windows))]
use mundy::{Interest, Preferences};
#[cfg(not(windows))]
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WindowEvent};

/// How long to wait for the system's answer; the window opens afterwards.
#[cfg(not(windows))]
const READ_TIMEOUT: Duration = Duration::from_millis(200);
#[cfg(not(windows))]
const CHANGED_EVENT: &str = "system-style-changed";

#[cfg(not(windows))]
pub(crate) struct StyleState(Mutex<SystemStyle>);

/// Tauri command `system_style`: the accent color and interface font last
/// read from the system.
#[cfg(not(windows))]
#[tauri::command]
pub fn system_style(state: tauri::State<'_, StyleState>) -> SystemStyle {
    state
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Tauri command `system_style`: empty on Windows, where the system
/// appearance carries the accent.
#[cfg(windows)]
#[tauri::command]
pub fn system_style() -> SystemStyle {
    SystemStyle::default()
}

/// Reads the style before the window shows and again whenever `window`
/// regains focus. Must run on the main thread.
#[cfg(not(windows))]
pub fn attach(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    app.manage(StyleState(Mutex::new(SystemStyle {
        accent: read_accent(),
        font: read_font(),
    })));
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(true)) {
            refresh(&app);
        }
    });
}

#[cfg(not(windows))]
fn refresh(app: &AppHandle) {
    // GTK and AppKit answer only on the main thread, where this runs.
    let font = read_font();
    #[cfg(target_os = "macos")]
    update(app, read_accent(), font);
    // The portal is a D-Bus call that may time out: keep it off the main
    // thread.
    #[cfg(not(target_os = "macos"))]
    {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || update(&app, read_accent(), font));
    }
}

#[cfg(not(windows))]
fn update(app: &AppHandle, accent: String, font: String) {
    let next = SystemStyle { accent, font };
    let state = app.state::<StyleState>();
    let mut current = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    if *current == next {
        return;
    }
    *current = next.clone();
    drop(current);
    if let Err(error) = app.emit(CHANGED_EVENT, next) {
        eprintln!("failed to send the system style: {error}");
    }
}

#[cfg(not(windows))]
fn read_accent() -> String {
    Preferences::once_blocking(Interest::AccentColor, READ_TIMEOUT)
        .and_then(|preferences| preferences.accent_color.0)
        .map(|color| accent_hex(color.red, color.green, color.blue))
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn read_font() -> String {
    use gtk::prelude::GtkSettingsExt;

    gtk::Settings::default()
        .and_then(|settings| settings.gtk_font_name())
        .and_then(|name| gtk::pango::FontDescription::from_string(&name).family())
        .map(|family| family.to_string())
        .unwrap_or_default()
}

#[cfg(all(not(windows), not(target_os = "linux")))]
fn read_font() -> String {
    String::new()
}

/// `#rrggbb` from sRGB channels in `0.0..=1.0`; values outside are clamped.
#[cfg(any(not(windows), test))]
fn accent_hex(red: f64, green: f64, blue: f64) -> String {
    let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(red),
        channel(green),
        channel(blue)
    )
}

#[cfg(test)]
mod tests {
    use super::accent_hex;

    #[test]
    fn formats_the_gnome_blue() {
        assert_eq!(
            accent_hex(0.207_843_14, 0.517_647_09, 0.894_117_65),
            "#3584e4"
        );
    }

    #[test]
    fn clamps_channels_out_of_range() {
        assert_eq!(accent_hex(-0.5, 1.5, 0.0), "#00ff00");
    }
}

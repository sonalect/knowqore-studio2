//! Which window buttons the custom title bar shows, and on which side.
//!
//! On Linux the desktop decides: GTK's `gtk-decoration-layout` setting, such
//! as `appmenu:close` on GNOME or `:minimize,maximize,close` on Ubuntu. On
//! other systems the buttons are minimize, maximize, and close on the right.

use serde::Serialize;

/// A window button of the title bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowControl {
    /// Minimizes the window.
    Minimize,
    /// Maximizes or restores the window.
    Maximize,
    /// Closes the window.
    Close,
}

/// The window buttons at each end of the title bar, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WindowControls {
    /// The buttons at the start (left) of the title bar.
    pub start: Vec<WindowControl>,
    /// The buttons at the end (right) of the title bar.
    pub end: Vec<WindowControl>,
}

impl Default for WindowControls {
    fn default() -> Self {
        Self {
            start: Vec::new(),
            end: vec![
                WindowControl::Minimize,
                WindowControl::Maximize,
                WindowControl::Close,
            ],
        }
    }
}

/// Tauri command `window_controls`: the buttons the title bar shows, read
/// from the desktop once at startup.
#[tauri::command]
pub fn window_controls(controls: tauri::State<'_, WindowControls>) -> WindowControls {
    controls.inner().clone()
}

/// The desktop's layout of window buttons. Must run on the main thread, where
/// GTK lives; a missing setting gives the default layout.
pub fn system_controls() -> WindowControls {
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::GtkSettingsExt;

        gtk::Settings::default()
            .and_then(|settings| settings.gtk_decoration_layout())
            .map(|layout| parse_layout(&layout))
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "linux"))]
    {
        WindowControls::default()
    }
}

/// Parses a GTK decoration layout: buttons before the colon go to the start,
/// after it to the end. Entries other than the three buttons (`appmenu`,
/// `icon`, `menu`, `spacer`) and repeated buttons are skipped.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn parse_layout(layout: &str) -> WindowControls {
    let (start, end) = layout.split_once(':').unwrap_or((layout, ""));
    let mut seen = Vec::new();
    let mut side = |names: &str| {
        names
            .split(',')
            .filter_map(|name| match name.trim() {
                "minimize" => Some(WindowControl::Minimize),
                "maximize" => Some(WindowControl::Maximize),
                "close" => Some(WindowControl::Close),
                _ => None,
            })
            .filter(|control| {
                let fresh = !seen.contains(control);
                seen.push(*control);
                fresh
            })
            .collect()
    };
    let start = side(start);
    let end = side(end);
    WindowControls { start, end }
}

#[cfg(test)]
mod tests {
    use super::{WindowControl, WindowControls, parse_layout};

    #[test]
    fn reads_the_ubuntu_layout() {
        assert_eq!(
            parse_layout("appmenu:minimize,maximize,close"),
            WindowControls::default()
        );
    }

    #[test]
    fn reads_the_gnome_layout_with_close_only() {
        assert_eq!(
            parse_layout("appmenu:close"),
            WindowControls {
                start: Vec::new(),
                end: vec![WindowControl::Close],
            }
        );
    }

    #[test]
    fn reads_buttons_on_the_left() {
        assert_eq!(
            parse_layout("close,minimize,maximize:"),
            WindowControls {
                start: vec![
                    WindowControl::Close,
                    WindowControl::Minimize,
                    WindowControl::Maximize,
                ],
                end: Vec::new(),
            }
        );
    }

    #[test]
    fn skips_unknown_and_repeated_entries() {
        assert_eq!(
            parse_layout("icon,close:spacer,close,maximize"),
            WindowControls {
                start: vec![WindowControl::Close],
                end: vec![WindowControl::Maximize],
            }
        );
    }

    #[test]
    fn a_layout_without_a_colon_is_all_at_the_start() {
        assert_eq!(
            parse_layout("close"),
            WindowControls {
                start: vec![WindowControl::Close],
                end: Vec::new(),
            }
        );
    }
}

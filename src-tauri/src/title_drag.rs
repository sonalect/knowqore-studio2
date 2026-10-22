//! Moving the window by its custom title bar on Linux.
//!
//! When the page starts the move, GTK takes the pointer and WebKit never sees
//! the button go up: it treats the next press as part of the old one, and
//! every second drag fails. So the press is taken before WebKit sees it: the
//! frontend reports where the title bar is and where its buttons are, and a
//! press on the bar outside the buttons starts the move (a double press
//! maximizes or restores) right in GTK, from the press event itself.

#[cfg(any(target_os = "linux", test))]
use serde::Deserialize;

/// A rectangle of the page, in CSS pixels from its top left corner.
#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Area {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(any(target_os = "linux", test))]
impl Area {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// The title bar and the buttons in it, which keep their clicks.
#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DragRegion {
    bar: Area,
    holes: Vec<Area>,
}

#[cfg(any(target_os = "linux", test))]
impl DragRegion {
    /// Whether a press at `x`, `y` moves the window.
    pub(crate) fn hits(&self, x: f64, y: f64) -> bool {
        self.bar.contains(x, y) && !self.holes.iter().any(|hole| hole.contains(x, y))
    }
}

/// The drag region the frontend last reported, shared with the GTK handler.
#[cfg(target_os = "linux")]
#[derive(Clone, Default)]
pub struct TitleDrag(std::sync::Arc<std::sync::Mutex<DragRegion>>);

#[cfg(target_os = "linux")]
impl TitleDrag {
    fn region(&self) -> std::sync::MutexGuard<'_, DragRegion> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Tauri command `set_title_drag_region`: where the custom title bar and its
/// buttons are now. Used on Linux only; elsewhere it does nothing.
#[cfg(target_os = "linux")]
#[tauri::command]
pub fn set_title_drag_region(region: DragRegion, drag: tauri::State<'_, TitleDrag>) {
    *drag.region() = region;
}

/// Tauri command `set_title_drag_region`: where the custom title bar and its
/// buttons are now. Used on Linux only; elsewhere it does nothing.
#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub fn set_title_drag_region() {}

/// Takes presses on the title bar of `window` before WebKit sees them. The
/// resize border of the undecorated window keeps its presses: its handler
/// runs first.
#[cfg(target_os = "linux")]
pub fn attach(window: &tauri::WebviewWindow, drag: TitleDrag) -> Result<(), String> {
    window
        .with_webview(move |webview| {
            use gtk::gdk::EventType;
            use gtk::glib::Propagation;
            use gtk::prelude::*;

            let drag = drag.clone();
            webview
                .inner()
                .connect_button_press_event(move |widget, event| {
                    let (x, y) = event.position();
                    if event.button() != 1 || !drag.region().hits(x, y) {
                        return Propagation::Proceed;
                    }
                    let Some(window) = widget
                        .toplevel()
                        .and_then(|top| top.downcast::<gtk::Window>().ok())
                    else {
                        return Propagation::Proceed;
                    };
                    match event.event_type() {
                        EventType::ButtonPress => {
                            let (root_x, root_y) = event.root();
                            window.begin_move_drag(1, root_x as i32, root_y as i32, event.time());
                        }
                        EventType::DoubleButtonPress => {
                            if window.is_maximized() {
                                window.unmaximize();
                            } else {
                                window.maximize();
                            }
                        }
                        _ => {}
                    }
                    Propagation::Stop
                });
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{Area, DragRegion};

    fn area(x: f64, y: f64, width: f64, height: f64) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }

    fn region() -> DragRegion {
        DragRegion {
            bar: area(0.0, 0.0, 800.0, 36.0),
            holes: vec![area(700.0, 6.0, 24.0, 24.0), area(760.0, 6.0, 24.0, 24.0)],
        }
    }

    #[test]
    fn a_press_on_the_bar_moves_the_window() {
        assert!(region().hits(300.0, 10.0));
    }

    #[test]
    fn a_press_on_a_button_keeps_its_click() {
        assert!(!region().hits(710.0, 15.0));
    }

    #[test]
    fn a_press_below_the_bar_is_the_page_own() {
        assert!(!region().hits(300.0, 36.0));
    }

    #[test]
    fn nothing_moves_the_window_before_the_bar_is_reported() {
        assert!(!DragRegion::default().hits(0.0, 0.0));
    }

    #[test]
    fn reads_the_frontend_report() {
        let region: DragRegion = serde_json::from_str(
            r#"{"bar":{"x":0,"y":0,"width":800,"height":36},"holes":[{"x":700,"y":6,"width":24,"height":24}]}"#,
        )
        .unwrap();
        assert_eq!(region.holes.len(), 1);
        assert!(region.hits(10.0, 10.0));
    }
}

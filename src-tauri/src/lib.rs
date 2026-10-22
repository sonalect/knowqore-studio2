mod lessons;
#[cfg(windows)]
mod snap_layout;
mod system_appearance;
mod system_locale;
mod system_style;
mod title_drag;
mod window_controls;
mod window_state;
mod workspace;

use mimalloc::MiMalloc;

// On the lib crate so the desktop binary, lib tests, and mobile
// cdylib/staticlib all share one allocator. Do not also set this in main.rs.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            lessons::lesson_facets,
            lessons::lesson_tree,
            lessons::lessons_page,
            system_appearance::system_appearance,
            system_locale::system_locale,
            system_style::system_style,
            title_drag::set_title_drag_region,
            window_controls::window_controls,
            workspace::open_workspace
        ])
        .setup(|app| {
            use tauri::Manager;
            workspace::manage(app.handle());
            lessons::manage(app.handle());
            app.manage(window_controls::system_controls());
            #[cfg(windows)]
            {
                system_appearance::watch(app.handle());
            }
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(windows)]
                {
                    system_appearance::apply_window_theme(&window);
                }
                window_state::attach(&window);
                #[cfg(not(windows))]
                system_style::attach(&window);
                #[cfg(target_os = "linux")]
                {
                    let drag = title_drag::TitleDrag::default();
                    app.manage(drag.clone());
                    if let Err(error) = title_drag::attach(&window, drag) {
                        eprintln!("failed to attach the title bar drag: {error}");
                    }
                }
                #[cfg(windows)]
                if let Err(error) = snap_layout::attach(&window) {
                    eprintln!("failed to attach Snap Layout overlay: {error}");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("error while running tauri application: {error}");
        std::process::exit(1);
    }
}

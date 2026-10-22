#[cfg(windows)]
mod snap_layout;
mod system_appearance;
mod system_style;
mod title_drag;
mod window_controls;
mod window_state;

use mimalloc::MiMalloc;

// On the lib crate so the desktop binary, lib tests, and mobile
// cdylib/staticlib all share one allocator. Do not also set this in main.rs.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            system_appearance::system_appearance,
            system_style::system_style,
            title_drag::set_title_drag_region,
            window_controls::window_controls
        ])
        .setup(|app| {
            use tauri::Manager;
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

mod autostart;
mod tray;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn is_autostart_enabled(app: tauri::AppHandle) -> tauri::Result<bool> {
    autostart::is_enabled(&app)
}

#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> tauri::Result<()> {
    autostart::set_enabled(&app, enabled)
}

/// Command wrapper to enable autostart. This forwards to the helper in
/// `autostart.rs` and is exposed to the frontend via `invoke('enable_autostart')`.
#[tauri::command]
fn enable_autostart(app: tauri::AppHandle) -> tauri::Result<()> {
    autostart::enable_autostart(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // build system tray and register event handler at builder level
    tauri::Builder::default()
        // register plugins before running setup so their managed state is
        // available (some plugins expose managed state accessed via
        // `app.autolaunch()` etc.). `init` requires a macOS launcher and an
        // optional args list; pass defaults here.
        .plugin(
            tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ),
        )
        // `opener` plugin not required for the minimal tray example — remove
        // unless you need external URL handling.
        .setup(|app| {
            // construct the tray using the new TrayIconBuilder API. `app.handle()`
            // returns an `AppHandle` while the closure receives `&mut App`.
            let handle = app.handle();
            tray::build_system_tray(&handle)?;

            // Hide the main window at startup so the app behaves as a tray-only
            // application. The window is declared in `tauri.conf.json`. We try to
            // get the main webview window and hide it now.
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.hide();
            }

            // Ensure the toggle-autostart menu item starts with the correct
            // checked state. Query the plugin and set the menu checkbox.
            let handle_clone = handle.clone();
            // We rely on the tray module to set the initial checked state for
            // the toggle item directly, so no action is required here. Keeping
            // this placeholder if you later prefer setting the menu via the
            // tray handle instead of the CheckMenuItem instance.
            let _ = handle_clone;

            Ok(())
        })
        // Intercept close requests on the main window and minimize to tray
        // instead of quitting. We call `prevent_close()` on the close event
        // and hide the window so the app remains running in the tray.
        .on_window_event(|window, event| {
            // Only handle the main window label
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            is_autostart_enabled,
            set_autostart_enabled
            ,enable_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

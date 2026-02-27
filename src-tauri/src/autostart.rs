use anyhow::Error as AnyhowError;
use tauri::Error as TauriError;
use tauri_plugin_autostart::ManagerExt;

// The tauri_plugin_autostart APIs return their own error type. Convert those
// errors into `tauri::Error` so the functions can return `tauri::Result` used
// by Tauri command handlers.
// Helper functions for autostart. These are called by command wrappers in
// `lib.rs`. We accept a borrowed `AppHandle` so callers don't need to move the
// handle.
pub fn is_enabled(app: &tauri::AppHandle) -> tauri::Result<bool> {
    let manager = app.autolaunch();
    manager
        .is_enabled()
        .map_err(|e| TauriError::from(AnyhowError::new(e)))
}

pub fn set_enabled(app: &tauri::AppHandle, enabled: bool) -> tauri::Result<()> {
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|e| TauriError::from(AnyhowError::new(e)))?;
    } else {
        manager
            .disable()
            .map_err(|e| TauriError::from(AnyhowError::new(e)))?;
    }
    Ok(())
}

// Lightweight helper to enable autostart. Exposed via a command wrapper in
// `lib.rs`.
pub fn enable_autostart(app: &tauri::AppHandle) -> tauri::Result<()> {
    let autostart_manager = app.autolaunch();
    autostart_manager
        .enable()
        .map_err(|e| TauriError::from(AnyhowError::new(e)))?;
    Ok(())
}

mod autostart;
mod network_commands;
mod network_logger;
mod overlay;
mod tray;

use network_commands::NetworkLoggerState;
use network_logger::NetworkLogger;
use serde_json::json;
use std::sync::Arc;
use sysinfo::{Networks, System};
use tauri::Manager;
use tokio::sync::Mutex;

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

#[tauri::command]
fn get_system_metrics() -> tauri::Result<serde_json::Value> {
    // Single-shot metrics snapshot. CPU measurements require two refreshes
    // separated by MINIMUM_CPU_UPDATE_INTERVAL to produce meaningful values.
    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

    sys.refresh_cpu_all();
    sys.refresh_memory();

    // Networks are now separate from System
    let mut networks = Networks::new_with_refreshed_list();
    networks.refresh(true);

    let cpu = sys.global_cpu_usage() as f64;
    let memory_pct = if sys.total_memory() > 0 {
        sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0
    } else {
        0.0
    };

    // Use per-refresh deltas for network (received()/transmitted()) and
    // divide by elapsed interval used above.
    let rx_bytes: u64 = networks.iter().map(|(_, d)| d.received()).sum();
    let tx_bytes: u64 = networks.iter().map(|(_, d)| d.transmitted()).sum();
    let elapsed = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.as_secs_f64().max(1e-6);
    let rx_kbps = (rx_bytes as f64 / elapsed) / 1024.0;
    let tx_kbps = (tx_bytes as f64 / elapsed) / 1024.0;

    let round1 = |v: f64| (v * 10.0).round() / 10.0;

    let payload = json!({
        "cpu": round1(cpu),
        "memory_pct": round1(memory_pct),
        "rx_kbps": round1(rx_kbps),
        "tx_kbps": round1(tx_kbps),
    });

    Ok(payload)
}

// Persisted overlay position is handled by the `tauri-plugin-sql` plugin
// from the frontend. We don't manage a direct sqlx pool in the Rust code
// to keep the backend lightweight.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // build system tray and register event handler at builder level
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // register plugins before running setup so their managed state is
        // available (some plugins expose managed state accessed via
        // `app.autolaunch()` etc.). `init` requires a macOS launcher and an
        // optional args list; pass defaults here.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // Register the positioner plugin so we can position windows
        // relative to the tray icon.
        .plugin(tauri_plugin_positioner::init())
        // SQL plugin: expose a guest-side API for lightweight DB access from
        // the frontend. We'll let the frontend `Database.load(...)` and run
        // `CREATE TABLE IF NOT EXISTS ...` on first load so we don't need to
        // manage migrations here.
        .plugin(tauri_plugin_sql::Builder::default().build())
        // Initialize network logger state
        .manage(NetworkLoggerState {
            logger: Arc::new(Mutex::new(None)),
        })
        // `opener` plugin not required for the minimal tray example — remove
        // unless you need external URL handling.
        .setup(|app| {
            // construct the tray using the new TrayIconBuilder API. `app.handle()`
            // returns an `AppHandle` while the closure receives `&mut App`.
            let handle = app.handle();

            // Initialize network logger
            let app_dir = handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;

            // Ensure directory exists
            if let Err(e) = std::fs::create_dir_all(&app_dir) {
                eprintln!("Failed to create app data dir: {}", e);
            }

            let db_path = app_dir.join("network_usage.db");

            // Log the database path for debugging
            eprintln!("Network logger database path: {:?}", db_path);

            let logger_state = handle.state::<NetworkLoggerState>();
            let logger_clone = logger_state.logger.clone();

            // Initialize logger in async context
            tauri::async_runtime::spawn(async move {
                match NetworkLogger::new(db_path).await {
                    Ok(logger) => {
                        let mut guard = logger_clone.lock().await;
                        *guard = Some(logger);
                        eprintln!("Network logger initialized successfully");
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize network logger: {}", e);
                    }
                }
            });

            // Use the `tauri-plugin-sql` plugin instead of managing an sqlx pool
            // directly. This reduces the number of heavy dependencies compiled in
            // the Tauri backend and exposes a simple API to the frontend.
            // The plugin will be configured (preloaded) via `tauri.conf.json`
            // or programmatically if needed. We still create the migrations
            // table here using the plugin's migration support when available.
            // For now, no additional state is managed here.
            // Ensure the positioner plugin is available to the tray builder
            // (positioner requires tray events when using tray-icon features).
            tray::build_system_tray(&handle)?;

            // Hide the main window at startup so the app behaves as a tray-only
            // application. The window is declared in `tauri.conf.json`. We try to
            // get the main webview window and hide it now.
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.hide();
            }

            // Create the overlay window (Windows only)
            overlay::create_overlay_window(&handle)?;

            Ok(())
        })
        // Intercept close requests on the main window and minimize to tray
        // instead of quitting. We call `prevent_close()` on the close event
        // and hide the window so the app remains running in the tray.
        .on_window_event(|window, event| {
            let label = window.label();

            // Handle main window
            if label == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }

            // Handle overlay window - delegate to overlay module
            if label == "overlay" {
                overlay::handle_overlay_event(window, event);
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            is_autostart_enabled,
            set_autostart_enabled,
            enable_autostart,
            get_system_metrics,
            network_commands::get_network_stats,
            network_commands::get_network_logs,
            network_commands::cleanup_network_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

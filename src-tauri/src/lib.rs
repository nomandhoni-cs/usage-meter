mod autostart;
mod tray;

use tauri::Manager;
use serde_json::json;
use sysinfo::{System, SystemExt, NetworksExt, NetworkExt, CpuExt};
use tauri::{WindowBuilder, WindowUrl, LogicalSize};

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
    std::thread::sleep(<sysinfo::System as SystemExt>::MINIMUM_CPU_UPDATE_INTERVAL);

    sys.refresh_cpu();
    sys.refresh_memory();
    sys.refresh_networks();

    let cpu = sys.global_cpu_info().cpu_usage() as f64;
    let memory_pct = if sys.total_memory() > 0 {
        sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0
    } else {
        0.0
    };

    // Use per-refresh deltas for network (received()/transmitted()) and
    // divide by elapsed interval used above.
    let rx_bytes: u64 = sys.networks().iter().map(|(_, d)| d.received()).sum();
    let tx_bytes: u64 = sys.networks().iter().map(|(_, d)| d.transmitted()).sum();
    let elapsed = <sysinfo::System as SystemExt>::MINIMUM_CPU_UPDATE_INTERVAL.as_secs_f64().max(1e-6);
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

            // On Windows create a small borderless overlay window that will be
            // used to display live metrics next to the taskbar. We create the
            // window programmatically so we can control decorations and
            // taskbar-skipping. Use a separate thread to avoid potential
            // WebView2 deadlocks when creating additional webviews during
            // startup.
            #[cfg(target_os = "windows")]
            {
                let app_handle = handle.clone();
                std::thread::spawn(move || {
                    // Size tuned for a compact two-line overlay. Adjust as
                    // desired.
                    let overlay_w = 240.0;
                    let overlay_h = 44.0;

                    // Load the bundled app entry but append a hash so the
                    // frontend can detect this is the overlay window and
                    // render a compact UI (`index.html#overlay`). Using
                    // `WindowUrl::App` makes this work in both dev and prod.
                    let url = WindowUrl::App("index.html#overlay".into());

                    // Build the window with minimal chrome and always-on-top.
                    // We don't attempt complex multi-monitor docking here; the
                    // frontend can request positioning or we can extend this
                    // logic later to compute the work area.
                    let _ = WindowBuilder::new(&app_handle, "overlay", url)
                        .title("timeman-overlay")
                        .inner_size(LogicalSize::new(overlay_w, overlay_h))
                        .decorations(false)
                        .skip_taskbar(true)
                        .always_on_top(true)
                        .transparent(true)
                        .visible(true)
                        .build();
                });
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
            set_autostart_enabled,
            enable_autostart,
            get_system_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

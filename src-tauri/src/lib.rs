mod autostart;
mod tray;

use tauri::Manager;
use serde_json::json;
use sysinfo::{System, SystemExt, NetworksExt, NetworkExt, CpuExt};
use tauri::{WebviewWindowBuilder, WebviewUrl};

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

// Persisted overlay position is handled by the `tauri-plugin-sql` plugin
// from the frontend. We don't manage a direct sqlx pool in the Rust code
// to keep the backend lightweight.

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
        // Register the positioner plugin so we can position windows
        // relative to the tray icon.
        .plugin(tauri_plugin_positioner::init())
        // SQL plugin: expose a guest-side API for lightweight DB access from
        // the frontend. We'll let the frontend `Database.load(...)` and run
        // `CREATE TABLE IF NOT EXISTS ...` on first load so we don't need to
        // manage migrations here.
        .plugin(tauri_plugin_sql::Builder::default().build())
        // `opener` plugin not required for the minimal tray example — remove
        // unless you need external URL handling.
        .setup(|app| {
            // construct the tray using the new TrayIconBuilder API. `app.handle()`
            // returns an `AppHandle` while the closure receives `&mut App`.
            let handle = app.handle();
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
                    // Set overlay width to 320 to reduce overflow while keeping compact
                    let overlay_w = 320.0;
                    // Compact height for single-line display
                    let overlay_h = 30.0;

                    // Load the bundled app entry but append a hash so the
                    // frontend can detect this is the overlay window and
                    // render a compact UI (`index.html#overlay`). Using
                    // `WebviewUrl::App` makes this work in both dev and prod.
                    // Use a dedicated overlay entry so the overlay webview loads
                    // `overlay.html` which hosts a minimal bundle.
                    let url = WebviewUrl::App("overlay.html".into());

                    // Compute a docked position using the primary monitor's
                    // work area so the overlay sits next to the taskbar. Try
                    // primary_monitor(), fall back to the first available
                    // monitor, and finally to a safe default. When the
                    // positioner plugin is available it will later reposition
                    // the overlay relative to the tray icon.
                    let margin = 8.0_f64;
                    let monitor_opt = match app_handle.primary_monitor() {
                        Ok(opt) => opt,
                        Err(_) => match app_handle.available_monitors() {
                            Ok(v) => v.into_iter().next(),
                            Err(_) => None,
                        },
                    };

                    let (pos_x, pos_y) = match monitor_opt {
                        Some(monitor) => {
                            let work = monitor.work_area();
                            let mon_pos = monitor.position();
                            let mon_size = monitor.size();
                            let scale = monitor.scale_factor();

                            // Convert physical -> logical pixels
                            let work_x = work.position.x as f64 / scale;
                            let work_y = work.position.y as f64 / scale;
                            let work_w = work.size.width as f64 / scale;
                            let work_h = work.size.height as f64 / scale;
                            let mon_x = mon_pos.x as f64 / scale;
                            let mon_y = mon_pos.y as f64 / scale;
                            let mon_w = mon_size.width as f64 / scale;
                            let mon_h = mon_size.height as f64 / scale;

                            // Detect which edge the taskbar occupies by
                            // comparing work area vs monitor bounds.
                            let mut _at_bottom = false;
                            let mut at_top = false;
                            let mut at_left = false;
                            let mut at_right = false;

                            if work_h < mon_h {
                                if work_y > mon_y {
                                    at_top = true;
                                } else {
                                    _at_bottom = true;
                                }
                            } else if work_w < mon_w {
                                if work_x > mon_x {
                                    at_left = true;
                                } else {
                                    at_right = true;
                                }
                            } else {
                                // Fallback to bottom if we can't detect.
                                _at_bottom = true;
                            }

                            let mut x = work_x + work_w - overlay_w - margin;
                            let mut y = work_y + work_h - overlay_h - margin;

                            if at_top {
                                y = work_y + margin;
                            } else if at_left {
                                x = work_x + margin;
                                y = work_y + work_h - overlay_h - margin;
                            } else if at_right {
                                x = work_x + work_w - overlay_w - margin;
                                y = work_y + work_h - overlay_h - margin;
                            }

                            // Clamp inside work area
                            let min_x = work_x + margin;
                            let max_x = (work_x + work_w - overlay_w - margin).max(min_x);
                            let min_y = work_y + margin;
                            let max_y = (work_y + work_h - overlay_h - margin).max(min_y);
                            if x < min_x {
                                x = min_x;
                            }
                            if x > max_x {
                                x = max_x;
                            }
                            if y < min_y {
                                y = min_y;
                            }
                            if y > max_y {
                                y = max_y;
                            }

                            (x, y)
                        }
                        None => {
                            // No monitor info available; fallback near origin.
                            (100.0, 100.0)
                        }
                    };

                    let _ = WebviewWindowBuilder::new(&app_handle, "overlay", url)
                        .title("timeman-overlay")
                        .inner_size(overlay_w, overlay_h)
                        .position(pos_x, pos_y)
                        .decorations(false)
                        .skip_taskbar(true)
                        .always_on_top(true)
                        .transparent(true)
                        .visible(true)
                        .build();
                });
            }

            // Ensure the toggle-autostart menu item starts with the correct
            // checked state. The tray module sets the initial state and also
            // forwards tray events to the positioner plugin so tray-relative
            // positions work.

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

use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{CpuExt, NetworkExt, NetworksExt, System, SystemExt};
use tauri::image::Image as TauriImage;
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_positioner::{Position, WindowExt};

// Build the system tray and register event handlers.
//
// Behavior implemented:
// - Single left-click on the tray icon toggles the main window:
//     * If the main window is hidden -> show, unminimize, focus
//     * If the main window is visible -> hide it (hide to tray)
// - The tray menu contains a Toggle Autostart item and Quit.
pub fn build_system_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Menu items (constructed with the v2 MenuBuilder API)
    // Create a checkable menu item so its checked state represents the
    // autostart enabled state.
    let toggle = CheckMenuItemBuilder::with_id("toggle-autostart", "Toggle Autostart")
        .checked(false)
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    // A menu item that toggles the main window's visibility. We'll set its
    // initial text to "Show" or "Hide" based on the current window state.
    let show_hide = MenuItemBuilder::with_id("toggle-window", "Show").build(app)?;

    // If the main window exists, set initial menu text to match its state.
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(visible) = window.is_visible() {
            let _ = show_hide.set_text(if visible { "Hide" } else { "Show" });
        }
    }

    let menu = MenuBuilder::new(app)
        .items(&[&toggle, &show_hide, &quit])
        .build()?;

    // Set initial checked state from the autostart plugin if available and
    // also emit an event so the frontend can synchronize on startup.
    if let Ok(enabled) = crate::autostart::is_enabled(app) {
        let _ = toggle.set_checked(enabled);
        // Emit an event so the frontend can update its UI without polling.
        let _ = app.emit("autostart-changed", enabled);
    }

    // Use the packaged icon for the tray. Prefer the Windows .ico when
    // available; fall back to a PNG if not. The icon files live in
    // `src-tauri/icons` and are included in the Tauri bundle by default.
    let icon_path = PathBuf::from("icons/icon.ico");
    // Try to load an Image from the packaged icons. If loading fails, fall
    // back to building the tray without an explicit icon (some platforms
    // will still show the app icon from resources).
    let maybe_icon = if icon_path.exists() {
        TauriImage::from_path(icon_path.clone()).ok()
    } else {
        TauriImage::from_path(PathBuf::from("icons/icon.png")).ok()
    };

    // Debounce state for click handling
    let last_click = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));
    let debounce_ms = Duration::from_millis(200);

    // We'll build the tray below; no intermediate TrayIconBuilder variable is
    // needed here to avoid type inference issues for the runtime generic.

    // Build the tray once and capture the returned `TrayIcon` so we can call
    // methods like `set_tooltip` on it later. The `TrayIconBuilder` is
    // consumed by `menu`/`on_tray_icon_event`, so we construct the builder anew
    // and immediately capture the built value into `tray`.
    let tray = if let Some(icon) = maybe_icon {
        TrayIconBuilder::new()
            .icon(icon)
            .show_menu_on_left_click(false)
            .menu(&menu)
            .on_tray_icon_event({
                let show_hide = show_hide.clone();
                let last_click = last_click.clone();
                move |tray, event| match event {
                    TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } => {
                        // Let the positioner plugin observe tray click events
                        // only — do not forward hover/move events that cause the
                        // overlay to reposition when the user merely moves the
                        // mouse over the tray icon.
                        let _ = tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

                        // Only handle mouse-up to match standard expectations.
                        if button_state != MouseButtonState::Up {
                            return;
                        }

                        // Debounce rapid clicks
                        {
                            let mut last = last_click.lock().unwrap();
                            if last.elapsed() < debounce_ms {
                                return;
                            }
                            *last = Instant::now();
                        }

                        if button == MouseButton::Left {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                if let Ok(visible) = window.is_visible() {
                                    if visible {
                                        let _ = window.hide();
                                        let _ = show_hide.set_text("Show");
                                        let _ = tray
                                            .set_tooltip(Some(String::from("Usage Meter — hidden")));
                                    } else {
                                        let _ = window.show();
                                        let _ = window.unminimize();
                                        let _ = window.set_focus();
                                        let _ = show_hide.set_text("Hide");
                                        let _ = tray
                                            .set_tooltip(Some(String::from("Usage Meter — visible")));
                                    }
                                } else {
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                    let _ = show_hide.set_text("Hide");
                                    let _ =
                                        tray.set_tooltip(Some(String::from("Usage Meter — visible")));
                                }
                            }
                        } else if button == MouseButton::Right {
                            // Let the system open the context menu on right click.
                        }

                        // Only reposition the overlay on explicit clicks — not
                        // on hover/move. A click indicates user intent to focus
                        // the app.
                        let app = tray.app_handle();
                        if let Some(overlay) = app.get_webview_window("overlay") {
                            let _ = overlay
                                .move_window_constrained(Position::TrayBottomCenter)
                                .or_else(|_| overlay.move_window_constrained(Position::TrayCenter));
                        }
                    }
                    _ => {
                        // For all non-click events (hover/move/enter/leave) do
                        // not forward to the positioner plugin to avoid the
                        // overlay following the mouse. This keeps the overlay
                        // stationary unless the user explicitly clicks the
                        // tray icon.
                        // NO-OP
                    }
                }
            })
            .on_menu_event({
                let toggle = toggle.clone();
                let show_hide = show_hide.clone();
                move |app, event| match event.id().as_ref() {
                    "toggle-autostart" => {
                        let app_handle = app.clone();
                        let toggle_clone = toggle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Ok(enabled) = crate::autostart::is_enabled(&app_handle) {
                                let _ = crate::autostart::set_enabled(&app_handle, !enabled);
                                let _ = toggle_clone.set_checked(!enabled);
                                let _ = app_handle.emit("autostart-changed", !enabled);
                            }
                        });
                    }
                    "toggle-window" => {
                        let app_handle = app.clone();
                        let show_hide_clone = show_hide.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                if let Ok(visible) = window.is_visible() {
                                    if visible {
                                        let _ = window.minimize();
                                        let _ = show_hide_clone.set_text("Show");
                                    } else {
                                        let _ = window.unminimize();
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                        let _ = show_hide_clone.set_text("Hide");
                                    }
                                }
                            }
                        });
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            })
            .build(app)?
    } else {
        TrayIconBuilder::new()
            .show_menu_on_left_click(false)
            .menu(&menu)
            .on_tray_icon_event({
                let show_hide = show_hide.clone();
                let last_click = last_click.clone();
                move |tray, event| match event {
                    TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } => {
                        if button_state != MouseButtonState::Up {
                            return;
                        }
                        {
                            let mut last = last_click.lock().unwrap();
                            if last.elapsed() < debounce_ms {
                                return;
                            }
                            *last = Instant::now();
                        }
                        if button == MouseButton::Left {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                if let Ok(visible) = window.is_visible() {
                                    if visible {
                                        let _ = window.hide();
                                        let _ = show_hide.set_text("Show");
                                        let _ = tray
                                            .set_tooltip(Some(String::from("Usage Meter — hidden")));
                                    } else {
                                        let _ = window.show();
                                        let _ = window.unminimize();
                                        let _ = window.set_focus();
                                        let _ = show_hide.set_text("Hide");
                                        let _ = tray
                                            .set_tooltip(Some(String::from("Usage Meter — visible")));
                                    }
                                } else {
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                    let _ = show_hide.set_text("Hide");
                                    let _ =
                                        tray.set_tooltip(Some(String::from("Usage Meter — visible")));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            })
            .on_menu_event({
                let toggle = toggle.clone();
                let show_hide = show_hide.clone();
                move |app, event| match event.id().as_ref() {
                    "toggle-autostart" => {
                        let app_handle = app.clone();
                        let toggle_clone = toggle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Ok(enabled) = crate::autostart::is_enabled(&app_handle) {
                                let _ = crate::autostart::set_enabled(&app_handle, !enabled);
                                let _ = toggle_clone.set_checked(!enabled);
                                let _ = app_handle.emit("autostart-changed", !enabled);
                            }
                        });
                    }
                    "toggle-window" => {
                        let app_handle = app.clone();
                        let show_hide_clone = show_hide.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                if let Ok(visible) = window.is_visible() {
                                    if visible {
                                        let _ = window.minimize();
                                        let _ = show_hide_clone.set_text("Show");
                                    } else {
                                        let _ = window.unminimize();
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                        let _ = show_hide_clone.set_text("Hide");
                                    }
                                }
                            }
                        });
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            })
            .build(app)?
    };

    // Set an initial tooltip so hovering the tray icon shows helpful text on
    // platforms that support it. Use an explicit generic to avoid type
    // inference issues across dependency crates.
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(visible) = window.is_visible() {
            let _ = tray.set_tooltip::<String>(Some(String::from(if visible {
                "Usage Meter — visible"
            } else {
                "Usage Meter — hidden"
            })));
        } else {
            let _ = tray.set_tooltip::<String>(Some(String::from("Usage Meter")));
        }
    } else {
        let _ = tray.set_tooltip::<String>(Some(String::from("Usage Meter")));
    }

    // ── Spawn metrics polling thread ─────────────────────────────────
    {
        let tray = tray.clone();
        let app_handle = app.clone();

        thread::spawn(move || {
            // Keep a single System instance for diffs
            let mut sys = System::new_all();
            sys.refresh_all();
            std::thread::sleep(<sysinfo::System as SystemExt>::MINIMUM_CPU_UPDATE_INTERVAL);

            // Initial totals for network delta calculation
            let mut prev_total_rx: u64 =
                sys.networks().iter().map(|(_, d)| d.total_received()).sum();
            let mut prev_total_tx: u64 = sys
                .networks()
                .iter()
                .map(|(_, d)| d.total_transmitted())
                .sum();

            loop {
                let tick_start = Instant::now();

                // Refresh required subsystems
                sys.refresh_cpu();
                sys.refresh_memory();
                sys.refresh_networks();

                // CPU %
                let cpu_percent = sys.global_cpu_info().cpu_usage() as f64;

                // Memory %
                let mem_pct = if sys.total_memory() > 0 {
                    sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0
                } else {
                    0.0
                };

                // Network totals & delta -> KB/s
                let total_rx: u64 = sys.networks().iter().map(|(_, d)| d.total_received()).sum();
                let total_tx: u64 = sys
                    .networks()
                    .iter()
                    .map(|(_, d)| d.total_transmitted())
                    .sum();

                let elapsed = tick_start.elapsed().as_secs_f64().max(1e-6);
                let rx_kbps = (total_rx.saturating_sub(prev_total_rx) as f64 / elapsed) / 1024.0;
                let tx_kbps = (total_tx.saturating_sub(prev_total_tx) as f64 / elapsed) / 1024.0;

                prev_total_rx = total_rx;
                prev_total_tx = total_tx;

                let tooltip = format!(
                    "CPU: {:.0}%  MEM: {:.0}%\n↓ {:.1} KB/s   ↑ {:.1} KB/s",
                    cpu_percent, mem_pct, rx_kbps, tx_kbps
                );

                let _ = tray.set_tooltip::<String>(Some(tooltip.clone()));

                #[cfg(target_os = "linux")]
                let _ = tray.set_title::<String>(Some(format!(
                    "{:.0}% • ↓{:.0}KB/s ↑{:.0}KB/s",
                    cpu_percent, rx_kbps, tx_kbps
                )));

                let payload = json!({
                    "cpu": (cpu_percent * 10.0).round() / 10.0,
                    "memory_pct": (mem_pct * 10.0).round() / 10.0,
                    "rx_kbps": (rx_kbps * 10.0).round() / 10.0,
                    "tx_kbps": (tx_kbps * 10.0).round() / 10.0,
                });
                let _ = app_handle.emit("metrics-updated", payload);

                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }

    Ok(())
}

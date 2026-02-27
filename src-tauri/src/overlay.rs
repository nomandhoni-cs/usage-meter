use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Initialize and create the overlay window on Windows
#[cfg(target_os = "windows")]
pub fn create_overlay_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let app_handle = app.clone();

    std::thread::spawn(move || {
        // Overlay window dimensions
        let overlay_w = 320.0;
        let overlay_h = 30.0;

        // Load the overlay HTML entry point
        let url = WebviewUrl::App("overlay.html".into());

        // Try to load saved position from database first
        let saved_pos = load_saved_position(&app_handle);

        let (pos_x, pos_y) = if let Some((x, y)) = saved_pos {
            // Use saved position from database
            (x, y)
        } else {
            // Calculate default position based on monitor and taskbar
            calculate_default_position(&app_handle, overlay_w, overlay_h)
        };

        // Create the overlay window
        let _ = WebviewWindowBuilder::new(&app_handle, "overlay", url)
            .title("usage-meter-overlay")
            .inner_size(overlay_w, overlay_h)
            .position(pos_x, pos_y)
            .decorations(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .transparent(true)
            .visible(true)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .closable(false)
            .focused(false)
            .build();
    });

    Ok(())
}

/// Placeholder for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn create_overlay_window(_app: &tauri::AppHandle) -> tauri::Result<()> {
    // Overlay is Windows-only for now
    Ok(())
}

/// Load saved overlay position from SQLite database
fn load_saved_position(app: &tauri::AppHandle) -> Option<(f64, f64)> {
    let app_dir = app.path().app_data_dir().ok()?;
    let db_path = app_dir.join("usage_meter.sqlite");

    // If database doesn't exist yet, return None (first run)
    if !db_path.exists() {
        eprintln!("Overlay position database not found at: {:?}", db_path);
        return None;
    }

    eprintln!("Loading overlay position from: {:?}", db_path);

    // Use blocking runtime to query the database synchronously
    let runtime = tokio::runtime::Runtime::new().ok()?;
    runtime.block_on(async {
        let connection_string = format!("sqlite://{}?mode=ro", db_path.display());
        let pool = sqlx::sqlite::SqlitePool::connect(&connection_string)
            .await
            .ok()?;

        // Query for saved position
        let result: Option<(f64, f64)> =
            sqlx::query_as("SELECT x, y FROM overlay_positions WHERE key = 'overlay'")
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();

        pool.close().await;

        if let Some((x, y)) = result {
            eprintln!("Loaded overlay position: x={}, y={}", x, y);
        } else {
            eprintln!("No saved overlay position found");
        }

        result
    })
}

/// Calculate default overlay position based on monitor and taskbar location
fn calculate_default_position(
    app: &tauri::AppHandle,
    overlay_w: f64,
    overlay_h: f64,
) -> (f64, f64) {
    let margin = 8.0_f64;

    // Try to get primary monitor, fall back to first available monitor
    let monitor_opt = match app.primary_monitor() {
        Ok(opt) => opt,
        Err(_) => match app.available_monitors() {
            Ok(v) => v.into_iter().next(),
            Err(_) => None,
        },
    };

    match monitor_opt {
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

            // Detect taskbar position by comparing work area vs monitor bounds
            let taskbar_at_top = work_h < mon_h && work_y > mon_y;
            let taskbar_at_left = work_w < mon_w && work_x > mon_x;
            let taskbar_at_right = work_w < mon_w && work_x <= mon_x;

            // Calculate position based on taskbar location
            let mut x = work_x + work_w - overlay_w - margin;
            let mut y = work_y + work_h - overlay_h - margin;

            if taskbar_at_top {
                y = work_y + margin;
            } else if taskbar_at_left {
                x = work_x + margin;
                y = work_y + work_h - overlay_h - margin;
            } else if taskbar_at_right {
                x = work_x + work_w - overlay_w - margin;
                y = work_y + work_h - overlay_h - margin;
            }

            // Clamp position inside work area
            let min_x = work_x + margin;
            let max_x = (work_x + work_w - overlay_w - margin).max(min_x);
            let min_y = work_y + margin;
            let max_y = (work_y + work_h - overlay_h - margin).max(min_y);

            x = x.clamp(min_x, max_x);
            y = y.clamp(min_y, max_y);

            (x, y)
        }
        None => {
            // No monitor info available; fallback near origin
            (100.0, 100.0)
        }
    }
}

/// Handle overlay window events to keep it always on top and visible
pub fn handle_overlay_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            // Prevent overlay from being closed
            api.prevent_close();
        }
        tauri::WindowEvent::Focused(false) => {
            // When overlay loses focus, ensure it stays on top and visible
            let _ = window.set_always_on_top(true);
            // Also ensure it's visible
            if let Ok(visible) = window.is_visible() {
                if !visible {
                    let _ = window.show();
                }
            }
        }
        tauri::WindowEvent::Moved(_) => {
            // When window is moved, ensure it stays on top
            let _ = window.set_always_on_top(true);
        }
        _ => {}
    }
}

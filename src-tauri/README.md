# Tauri Rust Backend (src-tauri)

This folder contains the minimal Rust backend for a tray-first Tauri app
example. It's intentionally small so you can use this repo as a boilerplate
template for building tray applications.

What this backend provides
- Tray-first UX: the main window is hidden at startup and the app runs in the
  system tray.
- System tray icon with a context menu and left-click show/hide behavior.
- Autostart integration using `tauri-plugin-autostart` with commands the
  frontend can invoke.

Important files
- `src/lib.rs` — App library entry. `run()` builds the `tauri::Builder`,
  registers plugins (autostart), constructs the system tray via `tray.rs`,
  hides the main window at startup, intercepts close requests (minimize to
  tray), and registers commands exposed to the frontend (`greet`,
  `is_autostart_enabled`, `set_autostart_enabled`, `enable_autostart`).
- `src/main.rs` — Small binary that calls `timeman_lib::run()`; keep this thin
  to reuse the same library entry point in other build targets.
- `src/tray.rs` — Builds the tray icon and menu using Tauri v2 APIs
  (`TrayIconBuilder`, `MenuBuilder`, `CheckMenuItemBuilder`). Handles
  left-click toggling of the main window and menu item actions (toggle
  autostart, show/hide, quit). Emits `autostart-changed` events so the
  frontend stays synchronized.
- `src/autostart.rs` — Lightweight wrappers around `tauri-plugin-autostart`.
  These helpers convert plugin errors into `tauri::Result` so they work cleanly
  with `#[tauri::command]` handlers and tray code.

How the pieces work together
- `run()` registers `tauri-plugin-autostart` before `.setup()` so the plugin's
  managed state is available via `app.autolaunch()`.
- `.setup()` calls `tray::build_system_tray(&handle)` and hides the `main`
  window so the app starts in the tray.
- When autostart is read or changed by the tray code, Rust emits
  `autostart-changed` events which the frontend can listen to and update UI.

Dev commands
- Start app (frontend + Rust): `bun run tauri dev` (runs Vite then the native
  app). See root README for full setup steps.
- Rust checks: `cargo check --manifest-path src-tauri/Cargo.toml`

Cleanup & minimalism
- This template aims to be minimal. Unused plugins and dependencies were
  removed from `src-tauri/Cargo.toml` (for example `opener`, `serde`,
  `serde_json`). Keep only what you need when customizing the template.

Customizing the template
1) Replace tray icons in `src-tauri/icons` and update `tauri.conf.json`.
2) Add frontend UI that calls commands: `invoke('is_autostart_enabled')`,
   `invoke('set_autostart_enabled', { enabled })`, `invoke('enable_autostart')`.
3) Emit or listen to more events from Rust (use `AppHandle.emit`) to
   synchronize state between tray and UI.

If you want, I can further trim the codebase, add CI, or prepare a publishable
boilerplate with a cleaned `Cargo.lock` and a concise contributor guide.

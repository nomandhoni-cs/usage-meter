<!--
  README for timeman: a tray-first Tauri + React template.
  Keep this file focused, easy to scan, and actionable for new contributors.
-->

# Usage Meter — compact, transparent taskbar overlay

[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![tauri](https://img.shields.io/badge/tauri-v2-blue.svg)](#)

Usage Meter is a lightweight tray-first desktop utility built with Tauri (Rust)
and a React (Vite) frontend. It provides a compact, always-on-top, fully
transparent taskbar overlay that shows CPU, RAM, and network throughput in a
single-line view. The project follows @context7 patterns and Tauri v2 best
practices for multi-window apps, updater signing, and tray UX.

Why this project

- Minimal, production-minded architecture for small background utilities.
- Transparent, borderless overlay webview that is draggable and persists
  position across launches (stored in SQLite via `@tauri-apps/plugin-sql`).
- Use of `tauri-plugin-positioner` for safe tray-relative placement and
  `tauri-plugin-updater` for signed update delivery.

Quick start (development)

Prerequisites

- Rust (recommended >= 1.77.2; pin with `rustup override set 1.88.0` if needed)
- Bun (preferred) or another Node-compatible package manager
- On Windows: Visual Studio Build Tools + Windows SDK (Desktop C++)

Run locally

1. Install frontend deps:

```bash
bun install
# or: npm install / pnpm install
```

2. Start dev (Vite + Tauri):

```bash
bun run tauri dev
# or: bun tauri dev
```

3. Interact with the overlay/tray:

- Left-click the tray icon to toggle the main window.
- Drag the overlay to reposition; position persists to `sqlite:usage_meter.sqlite`.

Build & release

- Frontend build: `bun run build` (this is wired into `src-tauri/tauri.conf.json` before build).
- Native build: `bun tauri build` or `cd src-tauri && cargo tauri build`.

Updater & signing (brief)

- Generate a signing key with the Tauri CLI: `bunx tauri signer generate -w ~/.tauri/myapp.key`.
- Add your private key as the `TAURI_SIGNING_PRIVATE_KEY` GitHub secret and the
  optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if you protect the key.
- Add the public key content to `src-tauri/tauri.conf.json` under
  `plugins.updater.pubkey`. The repository release workflow (`.github/workflows/release.yml`)
  automates building artifacts and attaching `latest.json` for static-updater usage.

Where to look

- Overlay UI: `src/overlay.tsx`, `src/overlay.css`, `src/overlay.html`.
- Main frontend: `src/main.tsx`, `src/App.tsx`.
- Rust backend: `src-tauri/src/lib.rs`, `src-tauri/src/tray.rs`.
- Tauri config: `src-tauri/tauri.conf.json`.

Contributing

- Small, focused PRs are preferred. Include a short verification checklist in
  the PR description (what you changed and how to test it locally).

License

MIT — see `LICENSE`.

Acknowledgements

- Follows patterns from @context7 and the Tauri v2 docs.

If you want I can add screenshots, examples for signing/CI, or turn the README
into a short developer guide — tell me which part to expand.

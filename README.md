# Usage Meter

<div align="center">
  <img src="src-tauri/icons/icon.svg" alt="Usage Meter Logo" width="120" height="120">
  
  <p><strong>A lightweight, transparent system monitoring overlay for Windows</strong></p>
  
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
  [![Tauri](https://img.shields.io/badge/Tauri-v2-blue.svg)](https://tauri.app)
  [![React](https://img.shields.io/badge/React-18-61dafb.svg)](https://reactjs.org)
  [![TypeScript](https://img.shields.io/badge/TypeScript-5-3178c6.svg)](https://www.typescriptlang.org)
</div>

---

## 📋 Overview

Usage Meter is a modern, tray-first desktop utility that provides real-time system monitoring through a compact, transparent overlay. Built with Tauri v2 (Rust) and React, it offers minimal resource usage while delivering comprehensive system metrics.

### ✨ Key Features

- **🖥️ Real-time System Monitoring**
  - CPU usage percentage
  - RAM usage percentage
  - Network upload/download speeds (KB/s, MB/s, GB/s)
  - Updates every second with minimal overhead

- **📊 Network Usage Tracking**
  - Automatic logging of network usage every minute
  - Historical data storage in SQLite database
  - View statistics by day, week, month, year, or custom date range
  - Detailed daily logs with upload/download breakdown

- **🎯 Transparent Overlay**
  - Always-on-top, borderless window
  - Draggable and repositionable
  - Position persistence across app restarts
  - Smart default positioning based on taskbar location

- **🔧 System Tray Integration**
  - Minimal tray icon with context menu
  - Toggle main settings window
  - Quick access to autostart settings
  - Clean exit option

- **⚙️ Settings & Configuration**
  - Autostart on system login
  - Built-in update checker with automatic downloads
  - Network usage history viewer
  - Clean, modern UI with dark/light mode support

---

## 🚀 Quick Start

### Prerequisites

- **Rust** (>= 1.77.2) - [Install Rust](https://rustup.rs/)
- **Bun** (recommended) or Node.js - [Install Bun](https://bun.sh/)
- **Windows** - Currently Windows-only (overlay feature)
- **Visual Studio Build Tools** (Windows) - Required for Rust compilation

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/nomandhoni-cs/usage-meter.git
   cd usage-meter
   ```

2. **Install dependencies**
   ```bash
   bun install
   # or: npm install / pnpm install / yarn install
   ```

3. **Run in development mode**
   ```bash
   bun run tauri dev
   ```

4. **Build for production**
   ```bash
   bun run tauri build
   ```

---

## 📖 Usage

### First Launch

1. The app starts minimized to the system tray
2. The overlay appears in the bottom-right corner (or near your taskbar)
3. Drag the overlay to your preferred position - it will remember this location

### Interacting with the App

- **Left-click tray icon** - Show/hide the main settings window
- **Right-click tray icon** - Open context menu
- **Drag overlay** - Reposition the overlay window
- **Settings window** - Configure autostart, check for updates, view network usage

### Network Usage Tracking

The app automatically tracks your network usage and stores it in a local SQLite database. View your usage statistics in the main window:

- **Today** - Current day's usage
- **This Week/Month/Year** - Aggregated statistics
- **Custom Range** - Select any date range
- **Daily Logs** - Detailed breakdown by day

---

## 🏗️ Architecture

### Project Structure

```
usage-meter/
├── src/                          # Frontend (React + TypeScript)
│   ├── components/               # React components
│   │   ├── NetworkUsage.tsx     # Network statistics viewer
│   │   └── NetworkUsage.css     # Component styles
│   ├── App.tsx                   # Main settings window
│   ├── overlay.tsx               # Overlay window component
│   └── main.tsx                  # App entry point
├── src-tauri/                    # Backend (Rust)
│   ├── src/
│   │   ├── lib.rs               # Main app setup
│   │   ├── overlay.rs           # Overlay window management
│   │   ├── tray.rs              # System tray implementation
│   │   ├── network_logger.rs   # Network usage tracking
│   │   ├── network_commands.rs # Network API commands
│   │   └── autostart.rs         # Autostart functionality
│   ├── Cargo.toml               # Rust dependencies
│   └── tauri.conf.json          # Tauri configuration
└── README.md
```

### Technology Stack

**Frontend:**
- React 18 with TypeScript
- Vite for fast development and building
- Tauri API for native integration
- CSS with dark/light mode support

**Backend:**
- Rust with Tauri v2 framework
- SQLx for database operations
- Sysinfo for system metrics
- Chrono for date/time handling

**Plugins:**
- `tauri-plugin-sql` - SQLite database access
- `tauri-plugin-autostart` - System startup integration
- `tauri-plugin-updater` - Automatic updates
- `tauri-plugin-process` - Process management
- `tauri-plugin-positioner` - Window positioning

---

## 🔧 Configuration

### Database

The app uses SQLite databases stored in the app data directory:

- **`usage_meter.sqlite`** - Overlay position storage
- **`network_usage.db`** - Network usage logs

**Location:** `%APPDATA%\com.usage.meter\` (Windows)

### Autostart

Enable autostart from the settings window or tray menu. The app will launch automatically when you log in to Windows.

### Updates

The app checks for updates from GitHub releases. When an update is available:
1. Click "Check for Updates" in the settings window
2. The app downloads and installs the update automatically
3. Restart to apply the update

---

## 🛠️ Development

### Running Tests

```bash
# Frontend tests
bun test

# Rust tests
cd src-tauri
cargo test
```

### Building

```bash
# Development build
bun run tauri dev

# Production build
bun run tauri build

# Build artifacts will be in src-tauri/target/release/bundle/
```

### Code Structure

**Modular Design:**
- Each feature is in its own module (overlay, tray, network logging)
- Clean separation between frontend and backend
- Type-safe communication via Tauri commands

**Key Modules:**
- `overlay.rs` - Independent overlay window management
- `tray.rs` - System tray with metrics polling
- `network_logger.rs` - Database operations for network tracking
- `network_commands.rs` - Tauri commands for frontend API

---

## 📦 Building for Release

### Prerequisites for Signing

1. Generate a signing key:
   ```bash
   bunx tauri signer generate -w ~/.tauri/usage-meter.key
   ```

2. Add to GitHub secrets:
   - `TAURI_SIGNING_PRIVATE_KEY` - Your private key content
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - Key password (if set)

3. Update `tauri.conf.json` with your public key

### Release Process

1. Update version in `package.json` and `src-tauri/Cargo.toml`
2. Commit and push changes
3. Create a new tag: `git tag v0.1.0 && git push --tags`
4. GitHub Actions will automatically build and create a release

---

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork the repository**
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Commit your changes** (`git commit -m 'Add amazing feature'`)
4. **Push to the branch** (`git push origin feature/amazing-feature`)
5. **Open a Pull Request**

### Code Style

- **Rust:** Follow `rustfmt` formatting
- **TypeScript:** Follow ESLint rules
- **Commits:** Use conventional commit messages

---

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgements

- Built with [Tauri](https://tauri.app/) - A framework for building desktop applications
- UI inspired by modern system monitoring tools
- Icons from [Ionicons](https://ionic.io/ionicons)
- Follows patterns from @context7 and Tauri v2 best practices

---

## 📧 Contact

**Project Link:** [https://github.com/nomandhoni-cs/usage-meter](https://github.com/nomandhoni-cs/usage-meter)

**Issues:** [https://github.com/nomandhoni-cs/usage-meter/issues](https://github.com/nomandhoni-cs/usage-meter/issues)

---

<div align="center">
  <p>Made with ❤️ using Rust and React</p>
  <p>⭐ Star this repo if you find it useful!</p>
</div>

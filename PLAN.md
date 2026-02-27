
# Implementation Plan — System Metrics in Tray (Tauri + Rust)

## Overview

Add a Rust-based metrics producer to the existing Tauri tray app that polls system metrics and exposes them to the UI.

- **Metrics:** overall CPU %, memory %, and network incoming/outgoing in KB/s.
- **Implementation** lives entirely in the Tauri backend (`src-tauri`). Polling runs in a background thread; updates are published both to the tray (tooltip / title) and as a JSON event (`metrics-updated`) for the React frontend.

---

## Goals

| Area | Detail |
|------|--------|
| **Tray tooltip** | Two-line block: `CPU: 12%  MEM: 34%` / `↓ 128.4 KB/s  ↑ 42.1 KB/s` |
| **Linux tray title** | Short one-liner (fallback because tooltips can be unreliable on some DEs) |
| **Frontend event** | JSON event `metrics-updated` emitted every tick so React can render a richer UI |
| **Cross-platform** | Windows / macOS / Linux via `sysinfo` crate |

---

## Step-by-Step Plan

### Step 1 — Add Dependencies

**File:** `src-tauri/Cargo.toml`

Add under `[dependencies]`:

```toml
sysinfo = "0.29"
serde_json = "1.0"
```

**Why these crates:**

| Crate | Purpose |
|-------|---------|
| `sysinfo` | Cross-platform CPU, memory, and network stats. Maintained, well-documented. |
| `serde_json` | Build JSON payloads for the `metrics-updated` event. |

---

### Step 2 — Update `src-tauri/src/tray.rs` (Primary Change)

This is the main edit. After the existing code that builds the `TrayIcon`, spawn a background thread that:

1. Maintains a **single** `sysinfo::System` instance (required by `sysinfo` for accurate diffs).
2. Refreshes CPU / memory / network each tick.
3. Computes `cpu%`, `memory%`, `rx_kbps`, `tx_kbps`.
4. Updates the tray tooltip/title.
5. Emits `metrics-updated` to the frontend.

#### 2a — Add Imports

Near the top of `tray.rs`, alongside existing `use` statements:

```rust
use sysinfo::{System, SystemExt};
use serde_json::json;
use std::thread;
use std::time::Instant;
```

> **Note:** Depending on the exact `sysinfo` version, you may also need `NetworkExt` / `NetworksExt`.

#### 2b — Spawn the Metrics Thread

Insert this block **after** the tray is built (after the `let tray = ...;` block) and **before** the final `Ok(())`:

```rust
// ── Metrics polling thread ──────────────────────────────────────────
{
    let tray = tray.clone();
    let app_handle = app.clone();

    thread::spawn(move || {
        // 1. Create & warm up System instance
        let mut sys = System::new_all();
        sys.refresh_all();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

        // 2. Capture initial network totals (for delta calculation)
        let mut prev_total_rx: u64 = sys
            .networks()
            .iter()
            .map(|(_, d)| d.total_received())
            .sum();
        let mut prev_total_tx: u64 = sys
            .networks()
            .iter()
            .map(|(_, d)| d.total_transmitted())
            .sum();

        // 3. Poll loop — runs every ~1 s
        loop {
            let tick_start = Instant::now();

            // Refresh only the subsystems we need
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            sys.refresh_networks();

            // ── CPU % ────────────────────────────────────────────
            let cpu = sys.global_cpu_usage() as f64;

            // ── Memory % ─────────────────────────────────────────
            let mem_pct = if sys.total_memory() > 0 {
                sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0
            } else {
                0.0
            };

            // ── Network KB/s (delta / elapsed) ───────────────────
            let total_rx: u64 = sys
                .networks()
                .iter()
                .map(|(_, d)| d.total_received())
                .sum();
            let total_tx: u64 = sys
                .networks()
                .iter()
                .map(|(_, d)| d.total_transmitted())
                .sum();

            let elapsed = tick_start.elapsed().as_secs_f64().max(1e-6);
            let rx_kbps =
                (total_rx.saturating_sub(prev_total_rx) as f64 / elapsed) / 1024.0;
            let tx_kbps =
                (total_tx.saturating_sub(prev_total_tx) as f64 / elapsed) / 1024.0;

            prev_total_rx = total_rx;
            prev_total_tx = total_tx;

            // ── Update tray tooltip ──────────────────────────────
            let tooltip = format!(
                "CPU: {:.0}%  MEM: {:.0}%\n\
                 ↓ {:.1} KB/s   ↑ {:.1} KB/s",
                cpu, mem_pct, rx_kbps, tx_kbps
            );
            let _ = tray.set_tooltip::<String>(Some(tooltip));

            // On Linux, also set the title (more reliable on some DEs)
            #[cfg(target_os = "linux")]
            let _ = tray.set_title::<String>(Some(format!(
                "{:.0}% • ↓{:.0}KB/s ↑{:.0}KB/s",
                cpu, rx_kbps, tx_kbps
            )));

            // ── Emit event to frontend ───────────────────────────
            let payload = json!({
                "cpu":        (cpu       * 10.0).round() / 10.0,
                "memory_pct": (mem_pct   * 10.0).round() / 10.0,
                "rx_kbps":    (rx_kbps   * 10.0).round() / 10.0,
                "tx_kbps":    (tx_kbps   * 10.0).round() / 10.0,
            });
            let _ = app_handle.emit_all("metrics-updated", payload);

            // ── Sleep until next tick ────────────────────────────
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}
```

---

### Step 3 (Optional) — Add On-Demand Tauri Command

**File:** `src-tauri/src/lib.rs`

If you also want the frontend to fetch metrics on demand (instead of only listening to events):

```rust
#[tauri::command]
fn get_system_metrics() -> Result<serde_json::Value, String> {
    // Similar logic to the polling thread but single-shot.
    // (Stub — implement when needed.)
    Err("not yet implemented".into())
}
```

Then register in the `invoke_handler!` macro:

```rust
.invoke_handler(tauri::generate_handler![get_system_metrics])
```

> This step is **not required** for the tray tooltip — it's an enhancement for richer frontend use cases.

---

### Step 4 — Frontend: Listen for `metrics-updated`

**File:** Any React component (e.g., `src/App.tsx` or a dedicated `MetricsWidget.tsx`)

```tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface Metrics {
  cpu: number;
  memory_pct: number;
  rx_kbps: number;
  tx_kbps: number;
}

export function useSystemMetrics() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    (async () => {
      unlisten = await listen<Metrics>("metrics-updated", (e) => {
        setMetrics(e.payload);
      });
    })();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return metrics;
}
```

Usage in a component:

```tsx
const metrics = useSystemMetrics();
// metrics?.cpu, metrics?.memory_pct, metrics?.rx_kbps, metrics?.tx_kbps
```

---

## Event Contract

| Field | Type | Unit | Example |
|-------|------|------|---------|
| `cpu` | `f64` | percent | `12.3` |
| `memory_pct` | `f64` | percent | `34.5` |
| `rx_kbps` | `f64` | KB/s | `128.4` |
| `tx_kbps` | `f64` | KB/s | `42.1` |

All values are rounded to one decimal place.

---

## Design Decisions & Rationale

| Decision | Why |
|----------|-----|
| **Single `System` instance in a long-lived thread** | `sysinfo` computes CPU usage as a delta between refreshes. Recreating the instance each time would yield 0% or meaningless values. |
| **`refresh_all()` + sleep on startup** | First CPU reading needs two data points; the initial sleep (≥ `MINIMUM_CPU_UPDATE_INTERVAL`) ensures the first real tick has valid data. |
| **`total_received()` deltas instead of `received()`** | `received()` gives bytes since last refresh, but the refresh interval isn't guaranteed to be exactly 1 s. Using total counters + our own `Instant` timer gives accurate KB/s. |
| **`saturating_sub` for network deltas** | Guards against counter wraps or interface resets. |
| **Best-effort tray updates (`let _ = ...`)** | Tray APIs can fail on certain platforms / DEs. We don't want the metrics thread to panic. |
| **`#[cfg(target_os = "linux")]` for `set_title`** | Tooltip support varies on Linux; `set_title` is a reliable fallback on some DEs. |
| **1 s polling interval** | Good balance between responsiveness and CPU overhead. Can be made configurable later. |

---

## Verification Steps

After applying the changes:

1. **Compile check:**
   ```bash
   cargo check --manifest-path src-tauri/Cargo.toml
   ```
2. **Run dev build:**
   ```bash
   bun run tauri dev
   # or: cargo tauri dev
   ```
3. **Verify tray tooltip** — hover over the tray icon; should show:
   ```
   CPU: 12%  MEM: 34%
   ↓ 128.4 KB/s   ↑ 42.1 KB/s
   ```
4. **Verify frontend event** — open DevTools console or use the `useSystemMetrics` hook; confirm `metrics-updated` payloads arrive every ~1 s.

---

## Caveats & Platform Notes

| Platform | Note |
|----------|------|
| **Windows** | `set_tooltip` works well; tooltip length limited to ~127 chars (our string is well within that). |
| **macOS** | `set_tooltip` supported. No `set_title` equivalent needed. |
| **Linux** | Tooltip support depends on DE / tray implementation. `set_title` added as fallback (shows text next to icon in some DEs). |
| **Binary size** | `sysinfo` adds ~1–2 MB to the final binary. Acceptable trade-off for cross-platform metrics. |
| **Polling frequency** | Lowering below 500 ms increases CPU usage from `sysinfo` itself. 1 s recommended. |

---

## Files Changed Summary

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `sysinfo` and `serde_json` dependencies |
| `src-tauri/src/tray.rs` | Add imports + spawn metrics polling thread |
| `src-tauri/src/lib.rs` | *(Optional)* Add `get_system_metrics` command |
| `src/hooks/useSystemMetrics.ts` | *(Optional)* React hook to consume `metrics-updated` |
```
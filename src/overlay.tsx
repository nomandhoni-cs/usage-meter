import { useEffect, useState, useRef } from "react";
import "./overlay.css";
import { listen } from "@tauri-apps/api/event";
// Note: we intentionally do not use `getCurrentWindow` here because
// checking `window.location.hash` is sufficient to determine whether the
// current webview is the overlay. This avoids extra async calls and the
// frontend permission error seen when a non-capable window attempts to
// `listen` to backend events.

type Metrics = {
  cpu: number;
  memory_pct: number;
  rx_kbps: number;
  tx_kbps: number;
};

export default function Overlay() {
  const [metrics, setMetrics] = useState<Metrics>({ cpu: 0, memory_pct: 0, rx_kbps: 0, tx_kbps: 0 });
  // Track dragging for possible future visual feedback; not used in layout now.
  const [, setDragging] = useState(false);
  const pollRef = useRef<number | null>(null);
  const lastPosRef = useRef<{ x: number; y: number } | null>(null);
  const lastChangeRef = useRef<number>(0);

  useEffect(() => {
    let unlisten: any = null;

    // Only listen when this code runs inside the overlay webview. We open
    // the overlay as a separate `overlay.html` so check the pathname.
    if (typeof window !== "undefined" && window.location && window.location.pathname.endsWith("overlay.html")) {
      listen("metrics-updated", (e: any) => {
        const p = e.payload as Metrics;
        setMetrics(p);
      }).then((fn) => (unlisten = fn));

      // Restore saved position from backend via invoke (command will be
      // and the plugin will be used to load the saved position.
      import('@tauri-apps/plugin-sql').then(async (mod) => {
        const Database = mod.default;
        try {
          const db = await Database.load('sqlite:usage_meter.sqlite');
          const rows: any[] = await db.select('SELECT x, y FROM overlay_positions WHERE key = $1', ['overlay']);
          if (rows && rows.length > 0) {
            const pos = rows[0];
            if (typeof pos.x === 'number' && typeof pos.y === 'number') {
              import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
                const win = getCurrentWindow();
                win.setPosition({ x: Math.round(pos.x), y: Math.round(pos.y) } as any).catch(() => {});
              });
            }
          }
          await db.close();
        } catch (e) {
          // ignore
        }
      }).catch(()=>{});
    }

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Start a drag when a stat element receives pointerdown. We add global
  // listeners so moves/up are tracked outside the overlay area and persist
  // the final position on pointerup.
  const startDrag = async (e: any) => {
    if (e.button !== 0) return;
    e.preventDefault();
    setDragging(true);

    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const win = getCurrentWindow();

      // Start native dragging; don't await so we can poll position while
      // the native drag occurs.
      try {
        // startDragging requires permission `core:window:allow-start-dragging` in capabilities
        // and will be a no-op if not permitted.
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        win.startDragging();
      } catch (err) {
        // Ignore if startDragging not available.
      }

      // Begin polling window position until it stabilizes, then persist.
      const pos = await win.outerPosition().catch(() => ({ x: 0, y: 0 } as any));
      lastPosRef.current = { x: pos.x, y: pos.y };
      lastChangeRef.current = Date.now();

      const pollMs = 150;
      const stableMs = 300;
      const maxMs = 10000;
      const startTs = Date.now();

      if (pollRef.current) {
        clearInterval(pollRef.current);
      }

      pollRef.current = window.setInterval(async () => {
        try {
          const p = await win.outerPosition();
          if (!lastPosRef.current || p.x !== lastPosRef.current.x || p.y !== lastPosRef.current.y) {
            lastPosRef.current = { x: p.x, y: p.y };
            lastChangeRef.current = Date.now();
          } else {
            if (Date.now() - lastChangeRef.current >= stableMs || Date.now() - startTs >= maxMs) {
              // position is stable; persist and stop polling
              try {
                const Database = (await import('@tauri-apps/plugin-sql')).default;
                const db = await Database.load('sqlite:usage_meter.sqlite');
                await db.execute('INSERT INTO overlay_positions (key, x, y) VALUES ($1, $2, $3) ON CONFLICT(key) DO UPDATE SET x=excluded.x, y=excluded.y', ['overlay', lastPosRef.current.x, lastPosRef.current.y]);
                await db.close();
              } catch (err) {
                // ignore persistence errors
              }
              setDragging(false);
              if (pollRef.current) {
                clearInterval(pollRef.current);
                pollRef.current = null;
              }
            }
          }
        } catch (err) {
          // ignore
        }
      }, pollMs);
    } catch (err) {
      // ignore
    }
  };

  return (
    <div className="wrap" onPointerDown={(e) => startDrag(e)}>
      <div className="stat">
        <div className="label">
          <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <rect x="6" y="6" width="12" height="12" rx="2" />
            <rect x="9" y="9" width="6" height="6" rx="1" />
            <path d="M9 3v2M15 3v2M9 19v2M15 19v2M3 9h2M3 15h2M19 9h2M19 15h2" />
          </svg>
          <span className="visually-hidden">CPU</span>
        </div>
        <div className="value">{metrics.cpu.toFixed(1)}%</div>
      </div>

      <div className="stat">
        <div className="label">
          <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <rect x="3" y="7" width="18" height="10" rx="2" />
            <path d="M7 11h10" />
            <path d="M7 14h10" />
            <path d="M4 21v-2M8 21v-2M12 21v-2M16 21v-2M20 21v-2" />
          </svg>
          <span className="visually-hidden">RAM</span>
        </div>
        <div className="value">{metrics.memory_pct.toFixed(0)}%</div>
      </div>

      <div className="sep">|</div>

      <div className="stat">
        <div className="label">
          <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M12 5v12" />
            <path d="M19 12l-7 7-7-7" />
          </svg>
          <span className="visually-hidden">RX</span>
        </div>
        <div className="value">{formatBandwidth(metrics.rx_kbps)}</div>
      </div>

      <div className="stat">
        <div className="label">
          <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M12 19V7" />
            <path d="M5 12l7-7 7 7" />
          </svg>
          <span className="visually-hidden">TX</span>
        </div>
        <div className="value">{formatBandwidth(metrics.tx_kbps)}</div>
      </div>
    </div>
  );
}

function formatBandwidth(kbps: number) {
  // kbps -> display as KB/s, MB/s, or GB/s with compact units
  if (kbps < 1024) return `${kbps.toFixed(1)}K/s`;
  const mb = kbps / 1024;
  if (mb < 1024) return `${mb.toFixed(1)}M/s`;
  return `${(mb / 1024).toFixed(1)}G/s`;
}

import { useEffect, useState } from "react";
import "./overlay.css";
import { listen } from "@tauri-apps/api/event";

type Metrics = {
  cpu: number;
  memory_pct: number;
  rx_kbps: number;
  tx_kbps: number;
};

export default function Overlay() {
  const [metrics, setMetrics] = useState<Metrics>({ cpu: 0, memory_pct: 0, rx_kbps: 0, tx_kbps: 0 });

  useEffect(() => {
    let unlisten: any = null;
    listen("metrics-updated", (e: any) => {
      const p = e.payload as Metrics;
      setMetrics(p);
    }).then((fn) => (unlisten = fn));

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return (
    <div className="wrap">
      <div className="top">
        <div className="big">{metrics.cpu.toFixed(1)}%</div>
        <div className="small">{metrics.memory_pct.toFixed(0)}%</div>
      </div>
      <div className="net">
        <div>↓ {metrics.rx_kbps.toFixed(1)} KB/s</div>
        <div>↑ {metrics.tx_kbps.toFixed(1)} KB/s</div>
      </div>
    </div>
  );
}

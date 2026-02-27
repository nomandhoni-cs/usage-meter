import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import NetworkUsage from "./components/NetworkUsage";
import "./App.css";

function App() {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [updateStatus, setUpdateStatus] = useState<string>("");
  const [isChecking, setIsChecking] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);

  useEffect(() => {
    // query current autostart value from backend
    invoke<boolean>("is_autostart_enabled")
      .then((v) => setAutostart(v))
      .catch(() => setAutostart(null));

    // subscribe to autostart change events emitted from Rust so the UI stays
    // in sync if the user toggles autostart via the tray menu
    let unlisten: (() => Promise<void>) | null = null;
    import("@tauri-apps/api/event")
      .then((event) => {
        const handler = (e: any) => setAutostart(Boolean(e.payload));
        return event.listen("autostart-changed", handler).then((fn: any) => {
          unlisten = fn;
        });
      })
      .catch(() => {
        /* ignore - running outside Tauri or event module missing */
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  async function toggleAutostart() {
    if (autostart === null) return;
    await invoke("set_autostart_enabled", { enabled: !autostart });
    setAutostart(!autostart);
  }

  async function checkForUpdates() {
    setIsChecking(true);
    setUpdateStatus("Checking for updates...");

    try {
      const update = await check();

      if (update?.available) {
        setUpdateStatus(`Update available: ${update.version}`);
        setIsDownloading(true);

        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case "Started":
              setUpdateStatus(`Downloading update (${event.data.contentLength} bytes)...`);
              break;
            case "Progress":
              setUpdateStatus(`Downloaded ${event.data.chunkLength} bytes`);
              break;
            case "Finished":
              setUpdateStatus("Download complete! Restarting...");
              break;
          }
        });

        await relaunch();
      } else {
        setUpdateStatus("You're on the latest version!");
      }
    } catch (error) {
      setUpdateStatus(`Update check failed: ${error}`);
    } finally {
      setIsChecking(false);
      setIsDownloading(false);
    }
  }

  return (
    <main className="container">
      <h1>Usage Meter</h1>
      <p className="subtitle">System monitoring overlay</p>

      <section className="settings-section">
        <h2>Settings</h2>

        <div className="setting-item">
          <div className="setting-info">
            <h3>Autostart</h3>
            <p>Launch Usage Meter when you log in</p>
            <p className="status">
              Status: {autostart === null ? "unknown" : autostart ? "enabled" : "disabled"}
            </p>
          </div>
          <button onClick={toggleAutostart} disabled={autostart === null}>
            {autostart ? "Disable" : "Enable"}
          </button>
        </div>

        <div className="setting-item">
          <div className="setting-info">
            <h3>Updates</h3>
            <p>Check for new versions of Usage Meter</p>
            {updateStatus && <p className="status">{updateStatus}</p>}
          </div>
          <button
            onClick={checkForUpdates}
            disabled={isChecking || isDownloading}
          >
            {isChecking ? "Checking..." : isDownloading ? "Downloading..." : "Check for Updates"}
          </button>
        </div>
      </section>

      <NetworkUsage />

      <footer>
        <p>The overlay window runs in the background. Check your system tray for options.</p>
      </footer>
    </main>
  );
}

export default App;

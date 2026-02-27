import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [autostart, setAutostart] = useState<boolean | null>(null);

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  useEffect(() => {
    // query current autostart value from backend
    invoke<boolean>("is_autostart_enabled")
      .then((v) => setAutostart(v))
      .catch(() => setAutostart(null));

    // subscribe to autostart change events emitted from Rust so the UI stays
    // in sync if the user toggles autostart via the tray menu. We listen for
    // `autostart-changed` events and update local state. If the event system
    // isn't used, this listener is effectively a no-op.
    let unlisten: (() => Promise<void>) | null = null;
    // Use dynamic import so bundlers don't include the event module for
    // non-Tauri builds. If the import fails (not running inside Tauri), we
    // silently ignore it.
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

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vite.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet</button>
      </form>
      <p>{greetMsg}</p>

      <section style={{ marginTop: 20 }}>
        <h2>Autostart</h2>
        <p>
          Auto startup is: {autostart === null ? "unknown" : autostart ? "enabled" : "disabled"}
        </p>
        <button onClick={toggleAutostart} disabled={autostart === null}>
          {autostart ? "Disable Autostart" : "Enable Autostart"}
        </button>
      </section>
    </main>
  );
}

export default App;

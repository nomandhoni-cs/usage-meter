import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Overlay from "./overlay";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
    {/* Render overlay component - it will be visible only in the overlay
        window because the window URL includes #overlay; the normal main
        window will ignore this content. Rendering here keeps the bundle
        simple for dev/prod. */}
    <Overlay />
  </React.StrictMode>,
);

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
// Overlay is provided as a separate HTML entry for the overlay webview.
// Do not render it inside the main window bundle.

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

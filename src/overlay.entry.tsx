import { createRoot } from "react-dom/client";
import Overlay from "./overlay";

const rootEl = document.getElementById("overlay-root");
if (rootEl) {
  createRoot(rootEl).render(<Overlay />);
}

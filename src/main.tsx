import React from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import App from "./App";
import { initAppearance, revealWindow } from "./appearance";
import "@astryxdesign/core/reset.css";
import "@astryxdesign/core/astryx.css";
import "@astryxdesign/theme-neutral/theme.css";
import "./index.css";

// The window starts hidden: show it after the first themed paint, or after
// three seconds if the appearance never arrives.
void (async () => {
  const revealTimeout = window.setTimeout(() => {
    void revealWindow();
  }, 3000);
  try {
    await initAppearance();
    const root = document.getElementById("root");
    if (root) {
      flushSync(() => {
        createRoot(root).render(
          <React.StrictMode>
            <App />
          </React.StrictMode>,
        );
      });
    }
  } finally {
    window.clearTimeout(revealTimeout);
    await revealWindow();
  }
})();

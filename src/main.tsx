import React from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import App from "./App";
import { initAppearance, revealWindow } from "./appearance";
import { loadWindowControls } from "./components/title-bar";
import "@astryxdesign/core/reset.css";
import "@astryxdesign/core/astryx.css";
import "@astryxdesign/theme-neutral/theme.css";
import "./index.css";

// The window starts hidden: show it once the themed page has had a frame to
// paint, or after three seconds if the appearance never arrives.
// Two animation frames: the first is the one that paints the rendered page.
// A hidden window may get no frames at all, so give up after 100 ms.
function nextFrame() {
  return new Promise<void>((resolve) => {
    const timeout = window.setTimeout(resolve, 100);
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        window.clearTimeout(timeout);
        resolve();
      });
    });
  });
}

void (async () => {
  const revealTimeout = window.setTimeout(() => {
    void revealWindow();
  }, 3000);
  try {
    const [, windowControls] = await Promise.all([
      initAppearance(),
      loadWindowControls(),
    ]);
    const root = document.getElementById("root");
    if (root) {
      flushSync(() => {
        createRoot(root).render(
          <React.StrictMode>
            <App windowControls={windowControls} />
          </React.StrictMode>,
        );
      });
      await nextFrame();
    }
  } finally {
    window.clearTimeout(revealTimeout);
    await revealWindow();
  }
})();

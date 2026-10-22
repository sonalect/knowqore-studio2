import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { isTauriRuntime } from "../appearance";

const WINDOW_TITLE = "Knowqore Studio";

function CaptionMinimizeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 12 12" className="caption-icon">
      <rect width="10" height="1" x="1" y="5.5" fill="currentColor" />
    </svg>
  );
}

function CaptionMaximizeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 12 12" className="caption-icon">
      <rect
        width="9"
        height="9"
        x="1.5"
        y="1.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
      />
    </svg>
  );
}

function CaptionRestoreIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 12 12" className="caption-icon">
      <path
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
        d="M3.5 3.5V1.5h7v7H8.5"
      />
      <rect
        x="1.5"
        y="3.5"
        width="7"
        height="7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
      />
    </svg>
  );
}

function CaptionCloseIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 12 12" className="caption-icon">
      <path
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.1"
        d="M2.5 2.5l7 7m0-7l-7 7"
      />
    </svg>
  );
}

const DOUBLE_CLICK_MS = 500;

/** The Windows 11 caption bar of the undecorated window on Windows. */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  const [snapHover, setSnapHover] = useState(false);
  const lastTitleClickAt = useRef(0);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    const appWindow = getCurrentWindow();
    let disposed = false;
    let unlistenResize: (() => void) | undefined;
    let unlistenHover: (() => void) | undefined;

    void (async () => {
      const nextMaximized = await appWindow.isMaximized();
      if (!disposed) {
        setMaximized(nextMaximized);
      }
      unlistenResize = await appWindow.onResized(async () => {
        setMaximized(await appWindow.isMaximized());
      });
      unlistenHover = await listen<boolean>("snap-layout-hover", (event) => {
        setSnapHover(event.payload);
      });
    })();

    return () => {
      disposed = true;
      unlistenResize?.();
      unlistenHover?.();
    };
  }, []);

  function minimize() {
    if (isTauriRuntime()) {
      void getCurrentWindow().minimize();
    }
  }

  function toggleMaximize() {
    if (isTauriRuntime()) {
      void getCurrentWindow().toggleMaximize();
    }
  }

  function close() {
    if (isTauriRuntime()) {
      void getCurrentWindow().close();
    }
  }

  function onTitlePointerDown(event: React.PointerEvent<HTMLDivElement>) {
    if (!isTauriRuntime() || event.button !== 0) {
      return;
    }

    event.preventDefault();

    const now = Date.now();
    const previousClickAt = lastTitleClickAt.current;
    lastTitleClickAt.current = now;

    if (now - previousClickAt < DOUBLE_CLICK_MS) {
      lastTitleClickAt.current = 0;
      toggleMaximize();
      return;
    }

    const originX = event.clientX;
    const originY = event.clientY;

    function onMove(moveEvent: PointerEvent) {
      if (
        Math.abs(moveEvent.clientX - originX) < 4 &&
        Math.abs(moveEvent.clientY - originY) < 4
      ) {
        return;
      }
      cleanup();
      void getCurrentWindow().startDragging();
    }

    function onUp() {
      cleanup();
    }

    function cleanup() {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    }

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }

  return (
    <header className="title-bar">
      <div className="title-bar__drag" onPointerDown={onTitlePointerDown}>
        <img src="/tauri.svg" alt="" className="title-bar__icon" />
        <span className="title-bar__title">{WINDOW_TITLE}</span>
      </div>
      <div className="caption-buttons">
        <button
          type="button"
          className="caption-button"
          aria-label="Minimize"
          onClick={minimize}
        >
          <CaptionMinimizeIcon />
        </button>
        <button
          type="button"
          className={
            snapHover
              ? "caption-button caption-button--active"
              : "caption-button"
          }
          aria-label={maximized ? "Restore" : "Maximize"}
          onClick={toggleMaximize}
        >
          {maximized ? <CaptionRestoreIcon /> : <CaptionMaximizeIcon />}
        </button>
        <button
          type="button"
          className="caption-button caption-button--close"
          aria-label="Close"
          onClick={close}
        >
          <CaptionCloseIcon />
        </button>
      </div>
    </header>
  );
}

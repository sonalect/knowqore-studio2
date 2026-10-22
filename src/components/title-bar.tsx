import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  type ReactNode,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { isTauriRuntime } from "../appearance";
import { platform } from "../platform";

const WINDOW_TITLE = "Knowqore Studio";

export type WindowControl = "minimize" | "maximize" | "close";

/** The window buttons at each end of the title bar, in order. */
export type WindowControls = {
  start: WindowControl[];
  end: WindowControl[];
};

const DEFAULT_CONTROLS: WindowControls = {
  start: [],
  end: ["minimize", "maximize", "close"],
};

/**
 * The window buttons the title bar shows. On Linux the desktop decides (the
 * `window_controls` command); elsewhere minimize, maximize, and close on the
 * right.
 */
export async function loadWindowControls(): Promise<WindowControls> {
  if (platform !== "linux" || !isTauriRuntime()) {
    return DEFAULT_CONTROLS;
  }
  try {
    return await invoke<WindowControls>("window_controls");
  } catch {
    return DEFAULT_CONTROLS;
  }
}

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

function LinuxMinimizeIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      className="window-control__icon"
    >
      <rect width="8" height="1.5" x="4" y="10.25" fill="currentColor" />
    </svg>
  );
}

function LinuxMaximizeIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      className="window-control__icon"
    >
      <rect
        width="7.5"
        height="7.5"
        x="4.25"
        y="4.25"
        rx="1"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
      />
    </svg>
  );
}

function LinuxRestoreIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      className="window-control__icon"
    >
      <path
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        d="M6.25 4.25h4.5a1 1 0 0 1 1 1v4.5"
      />
      <rect
        width="6"
        height="6"
        x="4.25"
        y="5.75"
        rx="1"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
      />
    </svg>
  );
}

function LinuxCloseIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      className="window-control__icon"
    >
      <path
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.5"
        d="M5 5l6 6m0-6l-6 6"
      />
    </svg>
  );
}

const DOUBLE_CLICK_MS = 500;

/**
 * The title bar of the undecorated window: Windows 11 caption buttons on
 * Windows, Adwaita-style round buttons on Linux. macOS keeps its own.
 */
export function TitleBar({ controls }: { controls: WindowControls }) {
  const [maximized, setMaximized] = useState(false);
  const [snapHover, setSnapHover] = useState(false);
  const lastTitleClickAt = useRef(0);
  const barRef = useRef<HTMLElement>(null);
  const reportedRegion = useRef("");

  // On Linux, tell the Rust side where the bar and its buttons are after
  // every render and resize: a press on the bar outside the buttons moves
  // the window there, before WebKit sees it.
  useLayoutEffect(() => {
    const bar = barRef.current;
    if (platform !== "linux" || !isTauriRuntime() || !bar) {
      return;
    }
    const reportDragRegion = () => {
      const area = (element: Element) => {
        const { x, y, width, height } = element.getBoundingClientRect();
        return { x, y, width, height };
      };
      const region = {
        bar: area(bar),
        holes: Array.from(
          bar.querySelectorAll(
            "button, a, input, select, textarea, [role='button']",
          ),
          area,
        ),
      };
      const key = JSON.stringify(region);
      if (key !== reportedRegion.current) {
        reportedRegion.current = key;
        void invoke("set_title_drag_region", { region });
      }
    };
    reportDragRegion();
    const observer = new ResizeObserver(reportDragRegion);
    observer.observe(bar);
    return () => observer.disconnect();
  });

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
      if (platform === "windows") {
        unlistenHover = await listen<boolean>("snap-layout-hover", (event) => {
          setSnapHover(event.payload);
        });
      }
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

  // On Linux the Rust side moves the window from the GTK press itself (see
  // `reportDragRegion`), so the page handles only Windows here.
  function onTitlePointerDown(event: React.PointerEvent<HTMLDivElement>) {
    if (platform !== "windows" || !isTauriRuntime() || event.button !== 0) {
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

  function windowsButton(control: WindowControl): ReactNode {
    switch (control) {
      case "minimize":
        return (
          <button
            key={control}
            type="button"
            className="caption-button"
            aria-label="Minimize"
            onClick={minimize}
          >
            <CaptionMinimizeIcon />
          </button>
        );
      case "maximize":
        return (
          <button
            key={control}
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
        );
      case "close":
        return (
          <button
            key={control}
            type="button"
            className="caption-button caption-button--close"
            aria-label="Close"
            onClick={close}
          >
            <CaptionCloseIcon />
          </button>
        );
    }
  }

  function linuxButton(control: WindowControl): ReactNode {
    switch (control) {
      case "minimize":
        return (
          <button
            key={control}
            type="button"
            className="window-control"
            aria-label="Minimize"
            onClick={minimize}
          >
            <LinuxMinimizeIcon />
          </button>
        );
      case "maximize":
        return (
          <button
            key={control}
            type="button"
            className="window-control"
            aria-label={maximized ? "Restore" : "Maximize"}
            onClick={toggleMaximize}
          >
            {maximized ? <LinuxRestoreIcon /> : <LinuxMaximizeIcon />}
          </button>
        );
      case "close":
        return (
          <button
            key={control}
            type="button"
            className="window-control"
            aria-label="Close"
            onClick={close}
          >
            <LinuxCloseIcon />
          </button>
        );
    }
  }

  const button = platform === "windows" ? windowsButton : linuxButton;
  const group = platform === "windows" ? "caption-buttons" : "window-controls";

  return (
    <header ref={barRef} className={`title-bar title-bar--${platform}`}>
      {controls.start.length > 0 ? (
        <div className={group}>{controls.start.map(button)}</div>
      ) : null}
      <div className="title-bar__drag" onPointerDown={onTitlePointerDown}>
        {platform === "windows" ? (
          <img src="/tauri.svg" alt="" className="title-bar__icon" />
        ) : null}
        <span className="title-bar__title">{WINDOW_TITLE}</span>
      </div>
      {controls.end.length > 0 ? (
        <div className={group}>{controls.end.map(button)}</div>
      ) : null}
    </header>
  );
}

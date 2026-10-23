import {
  colorVars,
  spacingVars,
  typographyVars,
} from "@astryxdesign/core/theme/tokens.stylex";
import * as stylex from "@stylexjs/stylex";
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

const styles = stylex.create({
  bar: {
    display: "flex",
    height: 32,
    flexShrink: 0,
    alignItems: "center",
    overflow: "hidden",
    color: colorVars["--color-text-primary"],
    userSelect: "none",
    WebkitUserSelect: "none",
  },
  linuxBar: {
    position: "relative",
    height: 36,
    borderBottomWidth: 1,
    borderBottomStyle: "solid",
    borderBottomColor: colorVars["--color-border-emphasized"],
  },
  drag: {
    display: "flex",
    height: "100%",
    minWidth: 0,
    flex: 1,
    alignItems: "center",
    gap: spacingVars["--spacing-2"],
    paddingBlock: 0,
    paddingInline: spacingVars["--spacing-3"],
  },
  icon: {
    width: spacingVars["--spacing-4"],
    height: spacingVars["--spacing-4"],
    pointerEvents: "none",
  },
  title: {
    overflow: "hidden",
    pointerEvents: "none",
    fontFamily: typographyVars["--font-family-body"],
    fontSize: "var(--text-supporting-size)",
    lineHeight: "var(--text-supporting-leading)",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  // GNOME centers the title over the whole bar, whatever the buttons.
  linuxTitle: {
    position: "absolute",
    left: "50%",
    transform: "translateX(-50%)",
    maxWidth: "60%",
    fontWeight: 600,
  },
  captionButtons: {
    display: "flex",
    height: 32,
    overflow: "hidden",
  },
  captionButton: {
    appearance: "none",
    display: "inline-flex",
    width: 46,
    height: 32,
    margin: 0,
    padding: 0,
    flexShrink: 0,
    alignItems: "center",
    justifyContent: "center",
    borderWidth: 0,
    borderRadius: 0,
    backgroundColor: {
      default: "transparent",
      ":hover": colorVars["--color-overlay-hover"],
    },
    boxShadow: "none",
    outline: "none",
    color: "inherit",
    cursor: "default",
    lineHeight: 0,
    overflow: "hidden",
  },
  // The maximize button while the Snap Layout overlay above it is hovered.
  captionButtonActive: {
    backgroundColor: colorVars["--color-overlay-hover"],
  },
  captionClose: {
    backgroundColor: {
      default: "transparent",
      ":hover": colorVars["--color-background-error-inverted"],
      ":active": colorVars["--color-background-error-inverted"],
    },
    color: {
      default: "inherit",
      ":hover": colorVars["--color-on-dark"],
      ":active": colorVars["--color-on-dark"],
    },
  },
  captionIcon: {
    width: spacingVars["--spacing-3"],
    height: spacingVars["--spacing-3"],
  },
  windowControls: {
    display: "flex",
    height: "100%",
    alignItems: "center",
    gap: spacingVars["--spacing-3"],
    paddingBlock: 0,
    paddingInline: spacingVars["--spacing-2"],
  },
  windowControl: {
    appearance: "none",
    display: "inline-flex",
    width: 24,
    height: 24,
    margin: 0,
    padding: 0,
    alignItems: "center",
    justifyContent: "center",
    borderWidth: 0,
    borderRadius: "50%",
    backgroundColor: {
      default: colorVars["--color-overlay-hover"],
      ":hover": colorVars["--color-overlay-pressed"],
      ":active": colorVars["--color-overlay-pressed"],
    },
    filter: {
      default: null,
      ":active": "brightness(0.9)",
    },
    outline: "none",
    color: "inherit",
    cursor: "default",
    lineHeight: 0,
  },
  windowControlIcon: {
    width: 16,
    height: 16,
  },
});

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
    <svg
      aria-hidden="true"
      viewBox="0 0 12 12"
      {...stylex.props(styles.captionIcon)}
    >
      <rect width="10" height="1" x="1" y="5.5" fill="currentColor" />
    </svg>
  );
}

function CaptionMaximizeIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 12 12"
      {...stylex.props(styles.captionIcon)}
    >
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
    <svg
      aria-hidden="true"
      viewBox="0 0 12 12"
      {...stylex.props(styles.captionIcon)}
    >
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
    <svg
      aria-hidden="true"
      viewBox="0 0 12 12"
      {...stylex.props(styles.captionIcon)}
    >
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
      {...stylex.props(styles.windowControlIcon)}
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
      {...stylex.props(styles.windowControlIcon)}
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
      {...stylex.props(styles.windowControlIcon)}
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
      {...stylex.props(styles.windowControlIcon)}
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
export function TitleBar({
  controls,
  trailing,
}: {
  controls: WindowControls;
  /** The app's own controls, between the title and the window buttons at the end. */
  trailing?: ReactNode;
}) {
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
            "button, a, input, select, textarea, [role='button'], [role='combobox']",
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
            {...stylex.props(styles.captionButton)}
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
            {...stylex.props(
              styles.captionButton,
              snapHover && styles.captionButtonActive,
            )}
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
            {...stylex.props(styles.captionButton, styles.captionClose)}
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
            {...stylex.props(styles.windowControl)}
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
            {...stylex.props(styles.windowControl)}
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
            {...stylex.props(styles.windowControl)}
            aria-label="Close"
            onClick={close}
          >
            <LinuxCloseIcon />
          </button>
        );
    }
  }

  const button = platform === "windows" ? windowsButton : linuxButton;
  const group = stylex.props(
    platform === "windows" ? styles.captionButtons : styles.windowControls,
  );
  const linux = platform === "linux";

  return (
    <header
      ref={barRef}
      {...stylex.props(styles.bar, linux && styles.linuxBar)}
    >
      {controls.start.length > 0 ? (
        <div {...group}>{controls.start.map(button)}</div>
      ) : null}
      <div {...stylex.props(styles.drag)} onPointerDown={onTitlePointerDown}>
        {platform === "windows" ? (
          <img src="/tauri.svg" alt="" {...stylex.props(styles.icon)} />
        ) : null}
        <span {...stylex.props(styles.title, linux && styles.linuxTitle)}>
          {WINDOW_TITLE}
        </span>
      </div>
      {trailing}
      {controls.end.length > 0 ? (
        <div {...group}>{controls.end.map(button)}</div>
      ) : null}
    </header>
  );
}

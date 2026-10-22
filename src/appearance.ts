import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { platform } from "./platform";

export type SystemAppearance = {
  accent: string;
  dark: boolean;
};

/** The accent color and interface font of Linux and macOS; empty when unknown. */
export type SystemStyle = {
  accent: string;
  font: string;
};

const NO_STYLE: SystemStyle = { accent: "", font: "" };

// Read before the first render, so the page starts in the system's style.
let initialStyle: SystemStyle = NO_STYLE;

export function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in globalThis;
}

// Only Windows reports its mode and accent through the `system_appearance`
// command. Elsewhere the page follows `prefers-color-scheme`, and the native
// title bar follows the system by itself.
const readsSystemAppearance = isTauriRuntime() && platform === "windows";

const darkQuery = "(prefers-color-scheme: dark)";

function applyDocumentTheme(isDark: boolean) {
  document.documentElement.classList.toggle("dark", isDark);
  document.documentElement.style.colorScheme = isDark ? "dark" : "light";
}

// The page's background as an opaque RGBA color, or null if it is not an
// opaque `rgb(...)` color.
function pageBackground(): [number, number, number, number] | null {
  const color = getComputedStyle(document.documentElement).backgroundColor;
  const match = /^rgb\((\d+), (\d+), (\d+)\)$/.exec(color);
  if (!match) {
    return null;
  }
  return [Number(match[1]), Number(match[2]), Number(match[3]), 255];
}

let syncedBackground = "";

/**
 * Outside Windows the native window and webview start white. Paints them
 * with the page's own background, so a frame the page has not drawn yet, or
 * the edge a resize uncovers, never flashes white. Call it once the Astryx
 * theme is applied; it does nothing while the color stays the same.
 */
export function syncNativeBackground() {
  if (!isTauriRuntime() || platform === "windows") {
    return;
  }
  const color = pageBackground();
  const key = color?.join(",") ?? "";
  if (color && key !== syncedBackground) {
    syncedBackground = key;
    void getCurrentWebviewWindow().setBackgroundColor(color);
  }
}

function applyAppearance(appearance: SystemAppearance) {
  applyDocumentTheme(appearance.dark);
  void getCurrentWindow().setTheme(appearance.dark ? "dark" : "light");
}

function followColorScheme(onChange: (dark: boolean) => void) {
  const media = window.matchMedia(darkQuery);
  const listener = (event: MediaQueryListEvent) => onChange(event.matches);
  media.addEventListener("change", listener);
  return () => media.removeEventListener("change", listener);
}

export function getPrefersDark() {
  return window.matchMedia(darkQuery).matches;
}

// Outside Windows the `system_style` command reports the accent and font.
const readsSystemStyle = isTauriRuntime() && platform !== "windows";

async function loadSystemStyle() {
  if (!readsSystemStyle) {
    return;
  }
  try {
    initialStyle = await invoke<SystemStyle>("system_style");
  } catch {
    initialStyle = NO_STYLE;
  }
}

export async function initAppearance() {
  applyDocumentTheme(getPrefersDark());
  await loadSystemStyle();
  // Before the Theme mounts this is the core Astryx shade; App syncs the
  // theme's own once it renders.
  syncNativeBackground();

  if (readsSystemAppearance) {
    try {
      applyAppearance(await invoke<SystemAppearance>("system_appearance"));
      await listen<SystemAppearance>("system-appearance-changed", (event) => {
        applyAppearance(event.payload);
      });
      return;
    } catch {
      // Fall through to the color scheme of the webview.
    }
  }
  followColorScheme(applyDocumentTheme);
}

export function useSystemAppearance(): SystemAppearance {
  const [appearance, setAppearance] = useState<SystemAppearance>(() => ({
    accent: "",
    dark: getPrefersDark(),
  }));

  useEffect(() => {
    const followScheme = () =>
      followColorScheme((dark) => {
        setAppearance((current) => ({ ...current, dark }));
      });

    if (!readsSystemAppearance) {
      return followScheme();
    }

    let unlisten: (() => void) | undefined;
    let disposed = false;

    void (async () => {
      try {
        const next = await invoke<SystemAppearance>("system_appearance");
        if (!disposed) {
          setAppearance(next);
        }
        const stop = await listen<SystemAppearance>(
          "system-appearance-changed",
          (event) => {
            setAppearance(event.payload);
          },
        );
        if (disposed) {
          stop();
        } else {
          unlisten = stop;
        }
      } catch {
        if (!disposed) {
          setAppearance({ accent: "", dark: getPrefersDark() });
          unlisten = followScheme();
        }
      }
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return appearance;
}

/**
 * The accent color and interface font of Linux and macOS, as read at startup
 * and each time the window regains focus. Empty on Windows.
 */
export function useSystemStyle(): SystemStyle {
  const [style, setStyle] = useState<SystemStyle>(() => initialStyle);

  useEffect(() => {
    if (!readsSystemStyle) {
      return;
    }
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void listen<SystemStyle>("system-style-changed", (event) => {
      setStyle(event.payload);
    }).then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return style;
}

export async function revealWindow() {
  if (!isTauriRuntime()) {
    return;
  }
  const current = getCurrentWindow();
  await current.show();
  await current.setFocus();
}

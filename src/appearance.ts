import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { platform } from "./platform";

export type SystemAppearance = {
  accent: string;
  dark: boolean;
};

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

export async function initAppearance() {
  applyDocumentTheme(getPrefersDark());

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

export async function revealWindow() {
  if (!isTauriRuntime()) {
    return;
  }
  const current = getCurrentWindow();
  await current.show();
  await current.setFocus();
}

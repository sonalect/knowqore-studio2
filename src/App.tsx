import { Center } from "@astryxdesign/core/Center";
import { StackItem, VStack } from "@astryxdesign/core/Stack";
import { Text } from "@astryxdesign/core/Text";
import { defineTheme, Theme } from "@astryxdesign/core/theme";
import { neutralTheme } from "@astryxdesign/theme-neutral";
import { useEffect, useMemo, useState } from "react";
import {
  syncNativeBackground,
  useSystemAppearance,
  useSystemStyle,
} from "./appearance";
import { TitleBar, type WindowControls } from "./components/title-bar";
import { drawsOwnTitleBar } from "./platform";
import { LessonsScreen } from "./screens/LessonsScreen";
import { OpenScreen } from "./screens/OpenScreen";
import { openWorkspace, type WorkspaceStatus } from "./workspace";

// Behind the system font, in case it is missing from the webview.
const FONT_FALLBACKS = "system-ui, sans-serif";

// Below this contrast ratio against white, text on the accent turns black.
const MIN_CONTRAST_WITH_WHITE = 3;

// White or black, whichever reads on `accent` (`#rrggbb`), as the system's own
// buttons do.
function onAccent(accent: string) {
  const linear = (offset: number) => {
    const value = Number.parseInt(accent.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  const luminance =
    0.2126 * linear(1) + 0.7152 * linear(3) + 0.0722 * linear(5);
  return 1.05 / (luminance + 0.05) >= MIN_CONTRAST_WITH_WHITE
    ? "#ffffff"
    : "#000000";
}

// The Neutral theme with the system's accent color and interface font, where
// the system reports them. Only the accent tokens change: seeding the palette
// with `color.accent` would also tint the backgrounds, borders, and text,
// which no system does. The accent stays the system's exact color, as on its
// own buttons.
function systemTheme(accent: string, font: string) {
  const hasAccent = /^#[0-9a-fA-F]{6}$/.test(accent);
  if (!hasAccent && !font) {
    return neutralTheme;
  }
  const role = { family: font, fallbacks: FONT_FALLBACKS };
  return defineTheme({
    name: "system",
    extends: neutralTheme,
    ...(hasAccent
      ? {
          tokens: {
            "--color-accent": accent,
            "--color-on-accent": onAccent(accent),
            // As Astryx derives them from a seeded accent.
            "--color-accent-muted": [
              "color-mix(in srgb, var(--color-accent) 20%, transparent)",
              "color-mix(in srgb, var(--color-accent) 25%, transparent)",
            ],
            "--color-text-accent": "var(--color-accent)",
            "--color-icon-accent": "var(--color-accent)",
          },
        }
      : {}),
    ...(font ? { typography: { body: role, heading: role } } : {}),
  });
}

// The status of opening `folder` (the default workspace when left out); a
// failed command reads as an unreadable folder, with its reason.
async function openOrReport(folder?: string): Promise<WorkspaceStatus> {
  try {
    return await openWorkspace(folder);
  } catch (error) {
    return {
      state: "unreadable",
      folder: folder ?? "the default workspace",
      reason: String(error),
    };
  }
}

function App({ windowControls }: { windowControls: WindowControls }) {
  const appearance = useSystemAppearance();
  const style = useSystemStyle();
  // Windows reports its accent with the appearance, Linux and macOS with the
  // style.
  const accent = appearance.accent || style.accent;
  // `null` while the workspace is being opened.
  const [workspace, setWorkspace] = useState<WorkspaceStatus | null>(null);
  const theme = useMemo(
    () => systemTheme(accent, style.font),
    [accent, style.font],
  );

  // After every render: the Theme below has applied its tokens to the page
  // by now, since children run their effects first.
  useEffect(() => {
    syncNativeBackground();
  });

  // At startup Studio opens the default workspace; only when that fails does
  // the person choose a folder.
  useEffect(() => {
    let isCurrent = true;
    void openOrReport().then((status) => {
      if (isCurrent) {
        setWorkspace(status);
      }
    });
    return () => {
      isCurrent = false;
    };
  }, []);

  async function open(folder: string) {
    setWorkspace(await openOrReport(folder));
  }

  return (
    <Theme theme={theme} mode={appearance.dark ? "dark" : "light"}>
      <VStack gap={0} height="100svh">
        {drawsOwnTitleBar ? <TitleBar controls={windowControls} /> : null}
        <StackItem size="fill">
          {workspace === null ? (
            <Center height="100%">
              <Text color="secondary">Opening the workspace…</Text>
            </Center>
          ) : workspace.state === "open" ? (
            <LessonsScreen workspace={workspace} />
          ) : (
            <OpenScreen status={workspace} onOpen={open} />
          )}
        </StackItem>
      </VStack>
    </Theme>
  );
}

export default App;

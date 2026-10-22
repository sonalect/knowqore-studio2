import { Button } from "@astryxdesign/core/Button";
import { Center } from "@astryxdesign/core/Center";
import { Heading } from "@astryxdesign/core/Heading";
import { Link } from "@astryxdesign/core/Link";
import { HStack, StackItem, VStack } from "@astryxdesign/core/Stack";
import { Text } from "@astryxdesign/core/Text";
import { TextInput } from "@astryxdesign/core/TextInput";
import { defineTheme, Theme } from "@astryxdesign/core/theme";
import { neutralTheme } from "@astryxdesign/theme-neutral";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import {
  syncNativeBackground,
  useSystemAppearance,
  useSystemStyle,
} from "./appearance";
import reactLogo from "./assets/react.svg";
import { AstryxLogo } from "./components/astryx-logo";
import { TitleBar, type WindowControls } from "./components/title-bar";
import { drawsOwnTitleBar } from "./platform";

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

function App({ windowControls }: { windowControls: WindowControls }) {
  const appearance = useSystemAppearance();
  const style = useSystemStyle();
  // Windows reports its accent with the appearance, Linux and macOS with the
  // style.
  const accent = appearance.accent || style.accent;
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const theme = useMemo(
    () => systemTheme(accent, style.font),
    [accent, style.font],
  );

  // After every render: the Theme below has applied its tokens to the page
  // by now, since children run their effects first.
  useEffect(() => {
    syncNativeBackground();
  });

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <Theme theme={theme} mode={appearance.dark ? "dark" : "light"}>
      <VStack gap={0} height="100svh">
        {drawsOwnTitleBar ? <TitleBar controls={windowControls} /> : null}
        <StackItem size="fill">
          <Center height="100%" padding={6}>
            <VStack gap={6} hAlign="center" maxWidth={640}>
              <Heading level={1}>Welcome to Knowqore Studio</Heading>
              <HStack gap={4} hAlign="center" wrap="wrap">
                <Link href="https://vite.dev" isExternalLink label="Vite">
                  <img src="/vite.svg" className="logo" alt="" />
                </Link>
                <Link href="https://tauri.app" isExternalLink label="Tauri">
                  <img src="/tauri.svg" className="logo" alt="" />
                </Link>
                <Link href="https://react.dev" isExternalLink label="React">
                  <img src={reactLogo} className="logo" alt="" />
                </Link>
                <Link
                  href="https://astryx.atmeta.com"
                  isExternalLink
                  label="Astryx"
                >
                  <AstryxLogo className="logo" />
                </Link>
              </HStack>
              <Text color="secondary">
                Click on the Tauri, Vite, React, and Astryx logos to learn more.
              </Text>
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  void greet();
                }}
              >
                <HStack gap={2} hAlign="center" vAlign="end">
                  <TextInput
                    label="Name"
                    placeholder="Enter a name..."
                    value={name}
                    onChange={setName}
                  />
                  <Button type="submit" variant="primary" label="Greet" />
                </HStack>
              </form>
              {greetMsg ? <Text>{greetMsg}</Text> : null}
            </VStack>
          </Center>
        </StackItem>
      </VStack>
    </Theme>
  );
}

export default App;

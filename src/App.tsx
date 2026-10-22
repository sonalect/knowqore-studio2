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
import { useMemo, useState } from "react";
import { useSystemAppearance } from "./appearance";
import reactLogo from "./assets/react.svg";
import { AstryxLogo } from "./components/astryx-logo";
import { TitleBar } from "./components/title-bar";
import { drawsOwnTitleBar } from "./platform";

function accentTheme(accent: string) {
  if (!/^#[0-9a-fA-F]{6}$/.test(accent)) {
    return neutralTheme;
  }
  return defineTheme({
    name: "system-accent",
    extends: neutralTheme,
    color: { accent },
  });
}

function App() {
  const appearance = useSystemAppearance();
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const theme = useMemo(
    () => accentTheme(appearance.accent),
    [appearance.accent],
  );

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <Theme theme={theme} mode={appearance.dark ? "dark" : "light"}>
      <VStack gap={0} height="100svh">
        {drawsOwnTitleBar ? <TitleBar /> : null}
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

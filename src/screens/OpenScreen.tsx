import { Button } from "@astryxdesign/core/Button";
import { Center } from "@astryxdesign/core/Center";
import { Heading } from "@astryxdesign/core/Heading";
import { HStack, StackItem, VStack } from "@astryxdesign/core/Stack";
import { Text } from "@astryxdesign/core/Text";
import { TextInput } from "@astryxdesign/core/TextInput";
import { useState } from "react";
import type { WorkspaceStatus } from "../workspace";

// Why the last folder did not open, in words for the person.
function reason(status: WorkspaceStatus): string {
  switch (status.state) {
    case "open":
      return "";
    case "noDefault":
      return "This system names no home folder, so Studio has no default place to look.";
    case "unreadable":
      return `Studio cannot read ${status.folder}: ${status.reason}`;
    case "noVault":
      return `There is no Knowqore Vault in ${status.folder}: neither data/.git nor .git is in it.`;
    case "refused":
      return `The store did not open ${status.folder} (${status.code}): ${status.message}`;
  }
}

function folderOf(status: WorkspaceStatus): string {
  return "folder" in status ? status.folder : "";
}

/**
 * Shown when the default workspace is missing or the store refuses it: the
 * person names the Knowqore home (or its data folder) explicitly.
 */
export function OpenScreen({
  status,
  onOpen,
}: {
  status: WorkspaceStatus;
  onOpen: (folder: string) => Promise<void>;
}) {
  const [folder, setFolder] = useState(() => folderOf(status));
  const [isOpening, setIsOpening] = useState(false);

  async function open() {
    setIsOpening(true);
    try {
      await onOpen(folder.trim());
    } finally {
      setIsOpening(false);
    }
  }

  return (
    <Center height="100%" padding={6}>
      <VStack gap={4} maxWidth={640}>
        <Heading level={1}>Open a Knowqore workspace</Heading>
        <Text color="secondary">{reason(status)}</Text>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void open();
          }}
        >
          <HStack gap={2} vAlign="end">
            <StackItem size="fill">
              <TextInput
                label="Workspace folder"
                placeholder="Path to the Knowqore home or its data folder"
                value={folder}
                onChange={setFolder}
              />
            </StackItem>
            <Button
              type="submit"
              variant="primary"
              label="Open"
              isDisabled={isOpening || folder.trim() === ""}
            />
          </HStack>
        </form>
      </VStack>
    </Center>
  );
}

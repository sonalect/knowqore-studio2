import { invoke } from "@tauri-apps/api/core";

/** What opening a workspace came to, as the `open_workspace` command reports it. */
export type WorkspaceStatus =
  | { state: "open"; home: string; vault: string }
  | { state: "noDefault" }
  | { state: "unreadable"; folder: string; reason: string }
  | { state: "noVault"; folder: string }
  | { state: "refused"; folder: string; code: string; message: string };

export type OpenWorkspace = Extract<WorkspaceStatus, { state: "open" }>;

/**
 * Opens `folder`, or the default location of this system when it is left out
 * (`~/.knowqore` on Linux and macOS, `%LOCALAPPDATA%\Knowqore` on Windows).
 */
export function openWorkspace(folder?: string): Promise<WorkspaceStatus> {
  return invoke<WorkspaceStatus>("open_workspace", { folder: folder ?? null });
}

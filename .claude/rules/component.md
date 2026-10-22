# Knowqore Studio: a read-only window on a Vault

This repository is Knowqore Studio: a desktop application that opens a
Knowqore workspace and shows what learning produced, Lessons first, to the
person who owns it. The screens are in the design canvas linked from
`README.md`; `DESIGN.md` holds the decisions and the stages.

## Read-only

Studio never writes to the Vault. It never claims the writer and never
calls a write of the contract (`Encode`, `Apply`, `Reindex`). A feature
that needs a write is out of scope: say so and stop.

## The contract is the only door

- Studio reads the Vault through the contract of `knowqore-stores`
  (`StoreSync`), and nothing else. Both crates of the store component,
  the contract `knowqore-stores` and its implementation
  `knowqore-store-scheda`, live in Knowqore's workspace (`rust/stores`,
  `rust/store-scheda`). Only the one module that opens a workspace names
  `knowqore-store-scheda` (its settings and `stores`, the library of
  stores); everything after it holds the store's sync form.
- Studio knows nothing about Scheda (owner, 12 October 2026). No crate
  here depends on `scheda`, not even in tests, and no code or comment
  names its index, files, or layout. Studio uses the store component
  only: the contract `knowqore-stores` and its implementation
  `knowqore-store-scheda`. What the store does (refusing a Vault of
  generation one, say) is tested in the store; Studio tests its own
  handling of the contract's replies and error codes.
- Studio does not read the files of the Vault, its `.git`, or the
  store's own index. What a screen needs and the contract lacks (the history
  of a Lesson, for one) is added to the contract in `knowqore-stores` and
  implemented in `knowqore-store-scheda` (owner, 12 October 2026), in
  Knowqore, as `upstream.md` says.
- The folder checks of the open screen may look at the folder itself
  (exists, readable, which of `data/` and the folder is the Vault) before
  the store opens. What the Vault declares comes from the store.

## Three systems

Linux, macOS, and Windows (owner, 12 October 2026). Code paths that
differ by system (the default workspace, file dialogs, keyboard shortcuts)
are written for all three at once, never "Linux now, the rest later".
There is no CI for now: the gate runs on Linux here.

## Two sides

Tauri 2. The Rust side (`src-tauri/`) owns the store and every read of the
Vault; the frontend (`src/`, React and TypeScript, built by Vite) draws the
screens. The frontend reaches the Vault only through Tauri commands
(`invoke`): never through a file, shell, or filesystem plugin. A Tauri
plugin or a capability in `src-tauri/capabilities/` that gives the webview
access to the disk or the shell is the owner's decision.

The design canvas is the reference for layout, wording, and states; a
deviation from it is the owner's decision.

## Threads

No store call runs on the main thread. A Tauri command that reads the
store is `async` (or hands the read to a blocking task), so the window
stays responsive while a large Vault loads. The frontend shows a pending
state while a command runs and the error it returns when it fails.

# Run commands from this repository

The repository has two roots, and every command runs in its own:

- `bun` commands (the frontend and the Tauri CLI): the clone root, the
  directory with `package.json`.
- `cargo` commands (the Rust side): `src-tauri/`, the directory with
  `Cargo.toml`. Or pass `--manifest-path src-tauri/Cargo.toml` from the
  clone root.

The Bash tool's working directory persists between calls and may have
drifted (another repository may be the session's primary directory), so
`cd` by absolute path in the same command when unsure. Never run them from
a sibling project, a scratch or tmp directory, or another subdirectory.

## Fast loop and gate

- `bun run tauri dev`: run Studio (Vite dev server and the Rust app). It
  does not exit by itself: run it only when asked, with the Bash tool's
  `run_in_background`, and stop it when done (`stoppable-work.md`).
- The gate, run before a commit (`commit-after-tests.md`):
  - `bun run build`: TypeScript check (`tsc`) and the Vite build;
  - `cargo test` in `src-tauri/`;
  - `cargo clippy --all-targets --all-features -- -D warnings` in
    `src-tauri/`;
  - `cargo fmt --check` in `src-tauri/`.
- `bun install` after a change to `package.json`; it updates `bun.lock`.

## No sandbox

Do not run `bun` or `cargo` inside a command sandbox. They need the network
(npm, crates.io, the Knowqore git dependency), and Cargo's cache lives under
`~/.cargo` and `src-tauri/target`. If Bash sandboxing is on, run these with
`dangerouslyDisableSandbox: true`.

# Run commands from this repository

Run every `bun` command from the clone root, the directory with
`package.json`. The scripts reach the Rust side with
`--manifest-path src-tauri/Cargo.toml`. A direct `cargo` command runs in
`src-tauri/`, the directory with `Cargo.toml`, or from the clone root with
`--manifest-path src-tauri/Cargo.toml`.

The Bash tool's working directory persists between calls and may have
drifted (another repository may be the session's primary directory), so
`cd` by absolute path in the same command when unsure. Never run them from
a sibling project, a scratch or tmp directory, or another subdirectory.

## Commands

| Command | What it does |
| - | - |
| `bun run check` | The gate before a commit (`commit-after-tests.md`): lint, build, Rust checks |
| `bun run lint` | Biome and markdownlint |
| `bun run build` | `tsc` and the Vite build |
| `bun run check:rust` | `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` |
| `bun run format` | Biome fixes and `cargo fmt` |
| `bun run tauri dev` | Run Studio (Vite dev server and the Rust app) |
| `bun install` | After a change to `package.json`; updates `bun.lock` |

`bun run tauri dev` does not exit by itself: run it only when asked, with
the Bash tool's `run_in_background`, and stop it when done
(`stoppable-work.md`).

`bun install` keeps the two-day quarantine by itself (`minimumReleaseAge`
in `bunfig.toml`); Cargo has no such setting (`dependency-quarantine.md`).

## No sandbox

Do not run `bun` or `cargo` inside a command sandbox. They need the network
(npm, crates.io, the Knowqore git dependency), and Cargo's cache lives under
`~/.cargo` and `src-tauri/target`. If Bash sandboxing is on, run these with
`dangerouslyDisableSandbox: true`.

# Bumps waiting out the quarantine

Checked 22 October 2026 (about 18:00 UTC), when every dependency was
pinned to an exact version and both lockfiles were refreshed.

Direct dependencies held back (`package.json`, `src-tauri/Cargo.toml`):

| Package | Pinned | Waiting | Published (UTC) | Allowed from (UTC) |
| - | - | - | - | - |
| `@tauri-apps/cli` (npm) | 2.12.3 | 2.12.4 | 21 Oct 12:20 | 23 Oct 12:20 |
| `vite` (npm) | 8.3.4 | 8.3.5 | 21 Oct 06:38 | 23 Oct 06:38 |
| `@biomejs/biome` (npm) | 2.5.15 | 2.6.2 | 21 Oct 11:08 | 23 Oct 11:08 |
| `markdownlint-cli2` (npm) | 0.23.3 | 0.24.0 | 21 Oct 06:59 | 23 Oct 06:59 |
| `tauri` | 2.12.4 | 2.12.5 | 21 Oct 11:54 | 23 Oct 11:54 |
| `tauri-build` | 2.7.3 | 2.7.4 | 21 Oct 11:53 | 23 Oct 11:53 |

Transitive crates held back in `src-tauri/Cargo.lock` with
`cargo update -p <crate> --precise <version>`:

| Crate | Locked | Waiting | Allowed from (UTC) |
| - | - | - | - |
| `tauri-codegen`, `tauri-macros` | 2.7.3 | 2.7.4 | 23 Oct 11:53 (come with `tauri` 2.12.5) |
| `tauri-runtime-wry` | 2.12.2 | 2.12.3 | 23 Oct 11:52 (comes with `tauri` 2.12.5) |
| `plist` | 1.10.2 | 1.10.3 | 23 Oct 12:36 |
| `open` | 5.4.4 | 5.4.5 | 24 Oct 03:52 |
| `icu_*` (seven crates) | 2.3.x | 2.4.0 | 24 Oct 08:56 |
| `tinystr`, `yoke`, `zerovec`, `zerovec-derive` | 0.8.4, 0.8.3, 0.11.8, 0.11.6 | 0.8.5, 0.8.4, 0.11.9, 0.11.7 | 24 Oct 08:51 |

`tokio` is locked at 1.54.0, a yanked version, since the store crates came
in (22 October 2026): Knowqore pins `tokio = "=1.54.0"`, and Cargo takes a
yanked version only by `cargo update -p tokio --precise 1.54.0`. It carries
RUSTSEC-2026-0384, a race in `tokio::time::Interval`, fixed in 1.54.1;
Studio moves when Knowqore does. Before the store, Studio had 1.54.1 by the
owner's exception.

Crates.io crates the store brings follow Knowqore's `Cargo.lock`. Two that
the first resolve took newer were set back to Knowqore's versions:
`pulldown-cmark` 0.13.4 and `arc-swap` 1.9.2. Entries that differ from
Knowqore's lock are other semver lines the Tauri stack needs (`syn` 1,
`base64` 0.21, `windows-sys` 0.45, …); Cargo keeps one version per line,
so they cannot match.

`@astryxdesign/core`, `@astryxdesign/theme-neutral`, and
`@astryxdesign/cli` are pinned at 0.7.3 (published 22 October 17:38–17:53
UTC), inside the quarantine, by the owner's exception (22 October 2026).
They were installed with `bun install
--minimum-release-age-excludes=@astryxdesign/core,@astryxdesign/theme-neutral,@astryxdesign/cli`;
their own dependencies stayed under the quarantine.

**Why:** the two-day rule (`dependency-quarantine.md`).

**How to apply:** after the dates above, bump the direct pins in
`package.json` and `src-tauri/Cargo.toml` (`tauri` and `tauri-build`
together),
then `bun install` (it keeps the quarantine through `bunfig.toml`) and
`cargo update` in `src-tauri/`. Check every `Cargo.lock` entry again: Cargo
has no release-age filter, so a refresh can pull versions younger than two
days; hold those back with `cargo update -p <crate> --precise <version>`,
dependents before their dependencies (`tinystr` before `zerovec`, `zerovec`
before `yoke`). Then run `bun run check`.

# Project notes

Project state, deferred work, measurements, and references that must
outlive one machine (see `.claude/rules/project-notes.md`). Read this index
before planning; open a note when its topic comes up.

| Note | What it holds |
| - | - |
| `quarantine.md` | Dependency bumps waiting out the two-day quarantine, and when each is allowed |
| `store-calls-wanted.md` | The paged and per-container Lesson calls the store lacks, which `lessons.rs` stands in for |

## References

- The design canvas:
  <https://claude.ai/artifact/MGS1xxjjj4F6fYyr24APHn#page-dcb81aee8d83>.
  Five artboards: the browser (`Main`), the open screen (`Open`), a folder
  that is not a workspace (`Validate`), the outcomes of the check
  (`States`), and the history events (`History`).
- The store Studio reads through: the crates `knowqore-stores` (the
  contract, `rust/stores`) and `knowqore-store-scheda` (its implementation,
  `rust/store-scheda`) of Knowqore, `github.com/sonalect/knowqore`
  (`../knowqore`), taken by a Cargo git `rev`. The separate repositories of
  the same names are closed (Knowqore's `docs/39` decision 36, 19 October
  2026).
- Knowqore itself: `github.com/sonalect/knowqore` (`../knowqore`): where
  the workspace lives by default (`~/.knowqore`, `%LOCALAPPDATA%\Knowqore`
  on Windows) and the encryption variables
  (`KNOWQORE_ENCRYPTION_PASSPHRASE`, `KNOWQORE_ENCRYPTION_IDENTITY_FILE`).
- The previous Studio, on GPUI and Bazel: `../knowqore-studio`. Its
  `DESIGN.md` and `README.md` hold the decisions and the behaviour of the
  screens (opening a workspace, the Lessons browser, the theme, the window
  state); its findings on the store (`protoc` for Lance's build scripts, the
  tokio pin of Knowqore) apply here too.
- UI stack: Tauri 2 (`tauri.app`), React, TypeScript, Vite; packages by
  `bun`.

# Changelog before commit

Before creating a git commit, make `CHANGELOG.md` **actual** for that
commit. Do not commit contract, behaviour, or
user-facing docs changes while `## [Unreleased]` still omits them.

## What to write

Keep a Changelog groups: `Added`, `Changed`, `Fixed`, `Removed`. English.
The user-facing *why* / effect, not a file list. New notes go under
`## [Unreleased]`. Do **not** bump the version in `src-tauri/Cargo.toml`,
`package.json`, or `src-tauri/tauri.conf.json` here: that is a release
(`.claude/skills/versioning/SKILL.md`).

If Unreleased already states the change, leave it. If it is stale or
wrong, edit it.

```text
# BAD — the open screen now refuses an old Vault, Unreleased empty
## [Unreleased]

# GOOD
## [Unreleased]
### Changed
- A Vault of an older format generation is refused with the store's
  words instead of an empty list.
```

## Skip (nothing notable)

- The commit is **only** `CHANGELOG.md`.
- rustfmt / comment-only / agent rules (`.claude/**`, `.cursor/**`) with
  no runtime or `DESIGN.md` change.
- Clippy / rustc / tsc / Biome / markdownlint lint-only: same runtime
  behaviour.
- The owner explicitly waived the changelog for this commit.

Do not skip for "small" changes to a screen, an error message, or what
Studio reads from the store. A lint fix that changes what the person sees
is not lint-only.

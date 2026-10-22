---
name: versioning
description: >-
  Bump the version of this repository in lockstep (Rust-style 0.x or
  SemVer after 1.0.0) across src-tauri/Cargo.toml, package.json, and
  tauri.conf.json, update CHANGELOG.md, create the matching GitHub tag,
  and publish the GitHub Release. Use when releasing, tagging, publishing a
  GitHub release, bumping version, or adding a CHANGELOG section.
---

# Versioning

One release version for the whole repository: the crate
`knowqore-studio`, the npm package, and the Tauri app.

## Scheme

Versions are `MAJOR.MINOR.PATCH`. `version = "0.0.1"` in Cargo,
`"version": "0.0.1"` in JSON have **no** `v`. The git tag **does**:
`v0.0.1`.

If **major is 0** (Rust approach, also stated in `CHANGELOG.md`):

- Breaking change → bump **minor** (`0.0.1` → `0.1.0`)
- Bugfix or compatible feature → bump **patch** (`0.0.1` → `0.0.2`)

If **major > 0** (standard SemVer):

- Breaking → **major**
- Compatible features → **minor**
- Bugfixes → **patch**

Do not skip numbers. Do not jump to `1.0.0` unless the user explicitly wants a
stable public API.

**Breaking** means visible to the person who runs Studio: a Vault it
opened before no longer opens, a screen or a command is gone, or a newer
`knowqore-stores` contract that older Vaults cannot meet. Toolchain
bumps (rustc, Tauri, bun) are a patch (0.x) or minor (≥1.0) unless they
force a breaking change.

## Lockstep files

Set the same `MAJOR.MINOR.PATCH` everywhere:

- `src-tauri/Cargo.toml`: `[package] version`
- `package.json`: `"version"`
- `src-tauri/tauri.conf.json`: `"version"`
- `CHANGELOG.md`: move `## [Unreleased]` notes into `## [MAJOR.MINOR.PATCH] -
  YYYY-MM-DD`; add the version to `## Links` and the reference definitions
  (`[Unreleased]` compare from the new tag, `[MAJOR.MINOR.PATCH]` release URL)

Refresh the crate's own entry in `src-tauri/Cargo.lock` (`cargo check` in
`src-tauri/`), and run `bun run check` (`commit-after-tests.md`).

## Release

After the version commit is on the default branch, create an annotated tag,
push it, and **publish a GitHub Release for that tag** (only when the user
asked to release). A tag without a published Release is incomplete. Do not
leave the Release as a draft. Do not mark it prerelease unless the user
asked. Do not backfill older tags.

```bash
git tag -a "vMAJOR.MINOR.PATCH" -m "vMAJOR.MINOR.PATCH"
git push origin "vMAJOR.MINOR.PATCH"
```

Release notes are the Keep a Changelog body for `## [MAJOR.MINOR.PATCH]` in
`CHANGELOG.md` (after that heading, until the next second-level heading,
so `## Links` stays out). Fail if that
heading is missing. Do not use `--generate-notes` or `--notes-from-tag`.

```bash
notes="$(mktemp)"
# The skill loader substitutes positional placeholders, so this block
# avoids them; escape each `.` of the version in the pattern.
sed -n '/^## \[MAJOR\.MINOR\.PATCH\]/,/^## /{/^## /d;p;}' CHANGELOG.md \
  | sed '/./,$!d' > "$notes"
test -s "$notes"

gh release create "vMAJOR.MINOR.PATCH" \
  --title "vMAJOR.MINOR.PATCH" \
  --notes-file "$notes" \
  --verify-tag
rm -f "$notes"
```

Run `git` / `gh` from the clone root with unrestricted permissions. Return
the Release URL when it succeeds.

Examples: `v0.0.1`, `v0.0.2`, `v0.1.0`.

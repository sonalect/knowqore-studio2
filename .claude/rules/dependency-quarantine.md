# Dependency quarantine

Before adding or bumping an **external** dependency in any language
(crates.io, npm, PyPI, Go modules, git tags, or any other registry), look
up the **publish time of the exact version**. Do not use a version
published less than **2 days** ago.

This is a quiet quarantine against freshly published malicious releases.

- Check the registry API or package page for that version. Do not trust
  memory, download mirrors, or lockfile dates. The `scout` agent can do
  the lookup.
- Take the newest version that has passed the quarantine and works with
  the rest of the build (owner, 22 October 2026). If the newest is too new,
  take the newest one at least 2 days old, or wait. Do not add the package
  if no qualifying version exists.
- A version held back for compatibility gets a comment at its pin saying
  what holds it, so the next check does not try it again.
- Applies to new packages and to version bumps of existing ones, git-tag
  pins of forks included, and to what a lockfile refresh (`bun install`,
  `bun update`, `cargo update`) pulls in. Does not apply to this repo's own
  path dependencies.
- Exception: every repository of the owner's GitHub organization
  `github.com/sonalect`, its own projects (`scheda`, `serde_markdown`,
  `bazel_utils`, `proto-contract.rs` with `protocontract`, every
  `knowqore-*` component) and its forks (`cxx`, `NumKong`, `USearch`,
  `rules_py`) alike. The owner writes and reviews them, so there is no
  third-party publisher to guard against: add or bump them at any time, a
  tag the day it ships. The same holds for the owner's personal account
  `github.com/amsokol` (the `mimalloc` fork). Any other repository stays
  under the two-day rule.
- crates.io crates: this crate takes each one at the version of Knowqore's
  `Cargo.lock`, which is already past the two-day check. A version Knowqore
  does not pin yet stays under the two-day rule.

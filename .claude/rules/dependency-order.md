# Dependency list order

When adding, removing, or bumping a dependency, put it in its sorted place.
Do not append at the bottom.

## Cargo.toml

`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, per-target
tables: two zones, decided by the owner (7 October 2026):

1. this repository's path crates (none yet), alphabetical;
2. third-party crates, alphabetical by package name.

Optional crates stay in the same sort, not in a block at the end.

```toml
# BAD — tokio dumped at the bottom
knowqore-stores = { git = "…", rev = "…" }
serde = { version = "1", features = ["derive"] }
tokio = "1"
serde_json = "1"

# GOOD
knowqore-stores = { git = "…", rev = "…" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = "1"
```

## package.json

`dependencies` and `devDependencies`: alphabetical by package name, scoped
packages (`@…`) first, as `bun add` writes them.

Does not replace `dependency-quarantine.md`.

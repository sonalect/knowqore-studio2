# Test before commit

Never create a git commit until the whole gate of `run-commands.md` has
exited 0 on **this working tree** in this session: `bun run build`, then
`cargo test`, `cargo clippy`, and `cargo fmt --check` in `src-tauri/`.
`cargo test` alone is not enough: a TypeScript error breaks
`bun run tauri build` just as a Rust one does.

Follow `run-commands.md`: each command in its root, unsandboxed.

```bash
# from the clone root
bun run build
cd src-tauri && cargo test \
  && cargo clippy --all-targets --all-features -- -D warnings \
  && cargo fmt --check
```

If it fails, fix and re-run. Do not commit a red tree.

```text
# BAD — commit after a local cargo test, skip the frontend build

# GOOD — the whole gate green, then git add / commit / push
```

Do not skip for "format-only", "comments", `tauri.conf.json`,
`package.json`, or `Cargo.toml`. The owner must explicitly waive this rule
to commit without the gate.

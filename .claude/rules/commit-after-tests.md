# Check before commit

Never create a git commit until `bun run check` has exited 0 on **this
working tree** in this session. It is the whole gate, in this order:

1. `bun run lint`: Biome (TypeScript, React, JSON) and markdownlint over
   every `*.md`, `.claude/` included;
2. `bun run build`: `tsc` and the Vite build;
3. `bun run check:rust`: `cargo test`, `cargo clippy -D warnings`, and
   `cargo fmt --check` for `src-tauri/`.

`cargo test` alone is not enough, and neither is `bun run build`: each
covers one side.

Follow `run-commands.md`: from the clone root, unsandboxed.

```bash
bun run check
```

If it fails, fix and re-run. `bun run format` fixes what Biome and rustfmt
can fix by themselves. Do not commit a red tree.

```text
# BAD — commit after a local cargo test, skip lint and the frontend build

# GOOD — bun run check green, then git add / commit / push
```

Do not skip for "format-only", "comments", "Markdown only",
`tauri.conf.json`, `package.json`, or `Cargo.toml`: the linters are part of
the gate. The owner must explicitly waive this rule to commit without it.

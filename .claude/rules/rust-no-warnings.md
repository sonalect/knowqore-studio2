---
paths:
  - "src-tauri/**/*.rs"
  - "src-tauri/Cargo.toml"
---

# No warnings by default

Rustc and Clippy warnings are not allowed. `bun run check` must be clean:
`cargo test`, `cargo clippy -D warnings`, `tsc`, and Biome.

Do not silence warnings with `allow` on hand-written code, and do not
silence TypeScript with `// @ts-ignore`, `// @ts-expect-error`, `any`, or
a `biome-ignore` comment to get past an error.

## Exception: generated files

Generated sources (Tauri's `gen/`, crate build scripts, `**/generated/**`)
may carry `#![allow(warnings)]` at the file root. Do not copy that allow
into handwritten `src/` or `tests/`.

```rust
// BAD — handwritten
#![allow(dead_code)]
#![allow(warnings)]

// GOOD — only at the top of a generated file
#![allow(warnings)]
```

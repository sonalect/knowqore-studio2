---
paths:
  - "src-tauri/**/*.rs"
  - "src-tauri/Cargo.toml"
---

# No warnings by default

Rustc and Clippy warnings are not allowed. `cargo test` and
`cargo clippy --all-targets --all-features -- -D warnings` must be clean.
`tsc` (`bun run build`) must be clean too.

Do not silence warnings with `allow` on hand-written code, and do not
silence TypeScript with `// @ts-ignore`, `// @ts-expect-error`, or `any`
to get past an error.

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

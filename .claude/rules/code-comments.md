---
paths:
  - "src-tauri/**/*.rs"
---

# Code comments are the API docs

Rustdoc is generated from comments. A reader of those pages does not have
the design package open. A pointer to a document is not an explanation.

## Coverage

Every **public** (`pub`) item on a handwritten crate surface needs rustdoc
(`///` or `//!`): crate, module, struct, enum, variant, union, trait,
associated type/const, `fn`, method, `const`, `static`, type alias, and
`pub` field.

`pub(crate)` and private items do not. Generated sources do not: they may
stay under `#![allow(warnings)]` at the file root.

When you add or change a public item, write the rustdoc in the same edit.
When you touch a public item that has none, add it. Do not rephrase an
existing comment unless you are already editing that item.

```rust
// BAD — public method, no rustdoc
pub fn code(&self) -> Code {
    self.code
}

// GOOD
/// The class of the failure, which a caller branches on.
pub fn code(&self) -> Code {
    self.code
}
```

## What the comment says

Do **not** cite `DESIGN.md`, its section numbers, or stage codes in
code comments (`//`, `///`, `//!`). Do not markdown-link them.

Put the **sense** in the comment: what the item does, the invariant, the
failure, the default. Take that text from the docs if needed; do not leave
the citation.

```rust
// BAD
/// Opens the workspace (§4.3, stage C2).
/// See DESIGN.md.

// GOOD
/// Opens the workspace at `path` read-only. A Vault of an older format
/// generation fails with the store's error code, never a panic.
```

Allowed in comments: public names, on-disk paths, error `code` strings,
Tauri command names. Those are the product, not a document index.

---
paths:
  - "src-tauri/**/*.rs"
---

# Rust panic policy

## Application code (`src-tauri/src/`)

Allowed only: `todo!`, `unimplemented!`, `unreachable!`.

Forbidden: `panic!`, `unwrap`, `expect`, `unwrap_err`, `unwrap_unchecked`,
`assert!`, `assert_eq!`, `assert_ne!`. Recoverable failure is `Result`
with the crate's own error type; a failure of a store call keeps the
`StoreErrorCode` of `knowqore-stores` it came with. A failure the person
can act on reaches the screen as a message, never as a crash.

```rust
// BAD
let id = map.get("id").unwrap();
panic!("missing id");

// GOOD
fn load_id(map: &Map) -> Result<&str, Error> {
    match map.get("id").and_then(Value::as_str) {
        Some(id) => Ok(id),
        None => Err(Error::new(StoreCode::MissingId, "missing id")),
    }
}

// GOOD — placeholder until the next stage implements it
todo!("functionality is not implemented")
```

## Tests (`src-tauri/tests/`, `#[cfg(test)]`)

`assert!`, `assert_eq!`, `assert_ne!`, `panic!`, `unwrap`, and `expect` are
allowed.

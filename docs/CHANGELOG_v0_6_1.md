# SARAKURA v0.6.1 changelog

v0.6.1 is a build-fix release for v0.6.

## Fixed

- Added the missing `pack_filter: vec![]` field to the `RunSummary` initializer in `sarakura-core/src/ci.rs` tests.
- Updated workspace version to `0.6.1`.

## Expected verification

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo test --workspace
```

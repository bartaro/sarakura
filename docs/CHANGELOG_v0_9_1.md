# SARAKURA v0.9.1 Changelog

## Fixed

- Fixed `sarakura-core/src/baseline.rs` build error caused by `diagnostic_map()` inferring `BTreeMap<String, AiDiagnostic>` instead of `BTreeMap<String, &AiDiagnostic>`.
- Added an explicit map type annotation and dereferenced the existing representative when comparing candidates.

## Compatibility

- No schema changes.
- No CLI behavior changes.
- v0.9 `baseline-delta` feature remains unchanged.

# SARAKURA v1.0.0 Release Notes

SARAKURA v1.0.0 is the first stable Rust workspace release for translating KITAQGB/KOKURA and KITAQFC/KUROSAKI runtime diagnostics into AI repair, retest, automation, CI, and repro-bundle artifacts.

Validated locally:

```bash
cargo fmt --all -- --check
cargo test --workspace
sarakura gb analyze --metadata examples/gb/e2e/metadata.json --events examples/gb/e2e/events.jsonl --out out/gb
sarakura fc analyze --metadata examples/fc/e2e/metadata.json --events examples/fc/e2e/events.jsonl --out out/fc
```


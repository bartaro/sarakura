# SARAKURA implementation notes

Generated: 2026-05-17

## Phase 0 delivered in this archive

- Workspace skeleton.
- Diagnostic catalog generated from v3 original specifications:
  - GB: 49 rules.
  - FC: 59 rules.
- Generic analyzer pipeline:
  - load build metadata
  - load diagnostic JSONL events
  - aggregate repeated events
  - correlate event -> source mapping
  - create evidence_chain
  - create target_candidates / repair_target
  - score confidence
  - write ai_diagnostics.json / summary / HTML / repro bundle

## Next implementation tasks

1. Run `cargo fmt` and `cargo test --workspace` in a Rust-enabled environment.
2. Fix any compile issues caused by dependency version drift.
3. Replace generic metadata correlation with strict GB/FC mappers per diagnostic category.
4. Add event emitters to KOKURA/KUROSAKI.
5. Add KITAQGB/KITAQFC metadata exporters.
6. Add real red/green ROM fixtures for MVP-1.

## Important design rule

Do not emit whole-instruction traces as JSON. `diagnostic_events.jsonl` must contain only violations or aggregated events.

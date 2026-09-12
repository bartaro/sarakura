# SARAKURA v0.5 changelog

Date: 2026-05-17

v0.5 turns v0.3 into a more practical bridge between KOKURA/KUROSAKI diagnostic emitters and SARAKURA reports.

## Added

- `retest_plan.json` is now emitted by default during `gb analyze` and `fc analyze`.
- `sarakura retest-plan --diagnostics <dir-or-ai_diagnostics.json>` command.
- `sarakura inspect-events --events <diagnostic_events.jsonl>` command.
- `sarakura inspect-metadata --metadata <build_metadata.json>` command.
- `sarakura emitter-check gb|fc --metadata <...> --events <...>` compatibility check.
- Repro bundle now includes `retest_plan.json` and links it from `manifest.json`.
- Core event inspection summaries: event counts, effective aggregated counts, severity counts, frame range, snapshot/trace reference counts.
- Core metadata inspection summaries: schema, build id, target, ROM path/hash, and metadata array counts.

## Improved

- Retest conditions are consolidated across diagnostics into a single retest plan.
- CLI smoke tests now cover inspect-events, retest-plan, and emitter-check.
- Analyzer retest output list now includes `retest_plan.json`.

## Intended v0.5 direction

- Strict GB/FC metadata mappers per diagnostic family.
- Real KOKURA/KUROSAKI event fixture ingestion tests.
- SARAKURA before/after HTML comparison report.
- JUnit/SARIF CI output.

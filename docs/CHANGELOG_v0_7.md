# SARAKURA v0.7 Changelog

## Added

- Added first-class `repair_plan.json` generation during `gb analyze` and `fc analyze`.
- Added `repair_plan.md` generation for human/Codex repair handoff.
- Added `sarakura repair-plan` command for regenerating a repair plan from an existing `ai_diagnostics.json`.
- Added `sarakura_repair_plan.schema.json`.
- Added repair-plan entries to `repro_bundle.zip` manifest and payload.
- Added unit tests for repair-plan grouping and markdown rendering.
- Added CLI smoke test for `repair-plan` output.

## Notes

v0.7 moves SARAKURA from diagnostics-only output toward an explicit repair workflow:

1. analyze emulator diagnostic events,
2. group diagnostics by safe repair target,
3. produce ordered repair steps,
4. retest using `retest_plan.json`.

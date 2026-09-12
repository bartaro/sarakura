# SARAKURA v0.9 Changelog

## Added

- Added `sarakura baseline-delta` command.
- Added structured `baseline_delta.json` and Markdown `baseline_delta.md` output.
- Added baseline/current comparison for new, resolved, persisting, regressed, and improved diagnostics.
- Added baseline gate policy with `--fail-on-new`, `--fail-on-regression`, and `--enforce`.
- Added `sarakura_baseline_delta.schema.json`.
- Added unit tests for baseline delta generation and CLI smoke coverage.

## Purpose

v0.9 turns SARAKURA into a CI-friendly regression guard. A project can keep a known baseline and fail only when new diagnostics or worse diagnostics are introduced.

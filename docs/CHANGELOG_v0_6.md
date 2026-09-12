# SARAKURA v0.6 changelog

v0.6 extends the v0.5 CI/event-normalization baseline with catalog coverage and diagnostic pack management.

## Added

- `sarakura coverage gb|fc --events <path>`.
- Catalog coverage JSON output for checking which SARAKURA rules were observed by KOKURA/KUROSAKI events.
- Unknown event type detection for emitter/schema drift.
- `sarakura pack-plan gb|fc`.
- Phase/domain/recommended diagnostic pack summaries.
- `--diagnostic-pack <pack>` filter for `gb analyze` and `fc analyze`.
- Core pack aliases: `core`, `MVP-1`, and domain names such as `ppu`, `audio`, `mapper`, `fds`.
- Unit tests for catalog coverage and pack matching.
- CLI smoke tests for `coverage`, `pack-plan`, and `--diagnostic-pack`.
- JSON schemas for `sarakura-catalog-coverage` and `sarakura-diagnostic-pack-plan`.

## Purpose

This version is aimed at real KOKURA/KUROSAKI integration. It helps answer:

- Which SARAKURA rules are actually exercised by the emulator emitter?
- Are there unknown event types that SARAKURA cannot translate yet?
- Which diagnostic pack should be enabled for a focused CI run?
- Can we restrict analysis to `core`, `ppu`, `audio`, `mapper`, `fds`, or phase-level packs?

#!/usr/bin/env bash
# Run from the repository root with Bash and Cargo. Stop on command failures,
# unset variables or failed pipeline components.
set -euo pipefail

# Export the two built-in catalogs to fixed temporary paths, replacing any
# previous files with those names.
cargo run -p sarakura-cli -- catalog gb >/tmp/sarakura_gb_catalog.md
cargo run -p sarakura-cli -- catalog fc --json >/tmp/sarakura_fc_catalog.json

# Generate GB reports from bundled event fixtures; no emulator or ROM is run.
# The never policy keeps expected fixture diagnostics from failing this example.
cargo run -p sarakura-cli -- gb analyze \
  --metadata examples/gb/kitaqgb_build_metadata.json \
  --events examples/gb/kokura_diagnostic_events.jsonl \
  --out out/gb \
  --fail-on never

# Repeat fixture analysis with the FC catalog and its own output directory.
cargo run -p sarakura-cli -- fc analyze \
  --metadata examples/fc/kitaqfc_build_metadata.json \
  --events examples/fc/kurosaki_diagnostic_events.jsonl \
  --out out/fc \
  --fail-on never

# Create a phase/severity-filtered view of the same GB events for comparison.
cargo run -p sarakura-cli -- gb analyze \
  --metadata examples/gb/kitaqgb_build_metadata.json \
  --events examples/gb/kokura_diagnostic_events.jsonl \
  --out out/gb_mvp1 \
  --phase MVP-1 \
  --min-severity warn \
  --fail-on never

# Validate the unfiltered document and compare it with the filtered view.
# The count difference comes from filtering, not from a repaired runtime.
cargo run -p sarakura-cli -- validate out/gb/ai_diagnostics.json
cargo run -p sarakura-cli -- compare --before out/gb --after out/gb_mvp1 --out out/compare_gb

#!/usr/bin/env bash
set -euo pipefail

cargo run -p sarakura-cli -- catalog gb >/tmp/sarakura_gb_catalog.md
cargo run -p sarakura-cli -- catalog fc --json >/tmp/sarakura_fc_catalog.json

cargo run -p sarakura-cli -- gb analyze \
  --metadata examples/gb/kitaqgb_build_metadata.json \
  --events examples/gb/kokura_diagnostic_events.jsonl \
  --out out/gb \
  --fail-on never

cargo run -p sarakura-cli -- fc analyze \
  --metadata examples/fc/kitaqfc_build_metadata.json \
  --events examples/fc/kurosaki_diagnostic_events.jsonl \
  --out out/fc \
  --fail-on never

cargo run -p sarakura-cli -- gb analyze \
  --metadata examples/gb/kitaqgb_build_metadata.json \
  --events examples/gb/kokura_diagnostic_events.jsonl \
  --out out/gb_mvp1 \
  --phase MVP-1 \
  --min-severity warn \
  --fail-on never

cargo run -p sarakura-cli -- validate out/gb/ai_diagnostics.json
cargo run -p sarakura-cli -- compare --before out/gb --after out/gb_mvp1 --out out/compare_gb

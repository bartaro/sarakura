# SARAKURA CLI v1.0

## Core Commands

```bash
sarakura gb analyze --metadata examples/gb/e2e/metadata.json --events examples/gb/e2e/events.jsonl --out out/gb
sarakura fc analyze --metadata examples/fc/e2e/metadata.json --events examples/fc/e2e/events.jsonl --out out/fc
sarakura validate out/gb/ai_diagnostics.json --strict
sarakura schema export --out schemas
sarakura inspect-repro out/gb/repro_bundle.zip --json
```

## CI Commands

```bash
sarakura ci-summary --diagnostics out/gb --fail-on error --enforce
sarakura baseline-delta --baseline out/baseline --current out/gb --out out/delta --markdown out/delta --enforce
```

## KOKURA / KUROSAKI Connection

```bash
sarakura emitter-check gb --metadata out/kitaqgb_build_metadata.json --events out/kokura_events.jsonl --strict
sarakura emitter-check fc --metadata out/kitaqfc_build_metadata.json --events out/kurosaki_events.jsonl --strict
sarakura automation-plan --diagnostics out/gb --tool kokura --capabilities out/kokura_capabilities.json --out out/gb
```


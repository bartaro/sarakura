# SARAKURA Troubleshooting

## `validate --strict` Fails

Check that `build_id`, `platform`, unique `diagnostic_id`, valid `severity`, `confidence` in `0.0..=1.0`, `repair_target.target_type`, and `retest_condition.expect_absent` are present.

## `emitter-check --strict` Fails

Run without `--strict` first and inspect the warnings. The most common issues are missing `event_id`, missing timing anchors, or missing `snapshot_ref` / `trace_window_ref`.

## Automation Commands Use Missing Emulator Options

Pass a capability file:

```json
{"tool":"kokura","commands":["run"],"options":["--frames","--json","--emit-diagnostics"]}
```

Then regenerate:

```bash
sarakura automation-plan --diagnostics out/gb --tool kokura --capabilities kokura_capabilities.json --out out/gb
```


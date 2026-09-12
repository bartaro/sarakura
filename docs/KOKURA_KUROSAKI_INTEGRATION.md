# KOKURA / KUROSAKI Integration

SARAKURA expects KOKURA and KUROSAKI to emit compact `diagnostic_events.jsonl` files. Each line is one violation or aggregate event, not a full CPU trace.

Minimum event fields:

- `schema`
- `schema_version`
- `event_id`
- `event_type`
- `severity`
- `frame` or `first_seen` / `last_seen`

Recommended event fields:

- `pc`
- `bank` or `prg_bank`
- `addr`
- `summary_key`
- `snapshot_ref`
- `trace_window_ref`

Artifact options shared by both emulators:

```bash
--emit-diagnostics out/events.jsonl
--diagnostics-jsonl out/events.jsonl
--png out/final.png
--snapshot out/final.snapshot.json
--trace-jsonl out/trace.jsonl
--repro-bundle out/repro_bundle.zip
--break-on-diagnostic all
--png-on-diagnostic out/screens
--snapshot-on-diagnostic out/snapshots
```

SARAKURA `automation-plan` accepts a capability JSON so generated commands avoid options that are not present in a local emulator build yet.


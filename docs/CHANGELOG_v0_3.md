# SARAKURA v0.3 変更メモ

作成日: 2026-05-17

## 目的

v0.2 のビルドエラー潰し前提版から、実際の KOKURA / KUROSAKI diagnostic event emitter と SARAKURA CLI をつなぐための実用機能を追加する。

## 主な変更

### CLI

- `sarakura catalog gb|fc` を追加。
- `sarakura catalog gb --json` / `sarakura catalog fc --json` を追加。
- `gb analyze` / `fc analyze` に `--diagnostic-rule` を追加。
  - event_type または catalog ID で絞り込み可能。
- `gb analyze` / `fc analyze` に `--phase` を追加。
  - `MVP-1` などで絞り込み可能。
- `gb analyze` / `fc analyze` に `--min-severity error|warn|info` を追加。
- `gb analyze` / `fc analyze` に `--fail-on never|error|warn|info` を追加。
  - CI で「errorが残ったら失敗」などを表現できる。

### ai_diagnostics.json

- `run` セクションを追加。
  - events_loaded
  - events_aggregated
  - diagnostics_before_filter
  - catalog_rules_total
  - filters
- `summary` を強化。
  - unmapped_diagnostics
  - phases
  - categories
  - repair_targets
- 各 diagnostic に以下を追加。
  - first_seen
  - last_seen
  - snapshot_ref
  - trace_window_ref

### evidence_chain

- runtime_event の note に count / first_seen / last_seen を追加。
- `snapshot_ref` がある場合は evidence_chain へ追加。
- `trace_window_ref` がある場合は evidence_chain へ追加。
- source_mapping の note に mapping confidence を追加。

### report / repair prompt

- HTML report に breakdown を追加。
  - phase別
  - category別
  - repair_target別
- HTML report に event_count / catalog_id / snapshot_ref / trace_window_ref を表示。
- repair_prompt.md を強化。
  - 修正順序
  - target_candidates
  - evidence_chain
  - retest condition

### repro bundle

- `manifest.json` を追加。
- bundle 内に platform / build_id / diagnostics_total / inputs を記録。

### validation

- `sarakura validate` を強化。
- diagnostics 各要素の必須項目を確認。
  - diagnostic_id
  - diagnostic_type
  - severity
  - confidence
  - repair_target
  - evidence_chain
  - retest_condition

## 想定コマンド

```bash
sarakura catalog gb
sarakura catalog fc --json

sarakura gb analyze \
  --metadata examples/gb/kitaqgb_build_metadata.json \
  --events examples/gb/kokura_diagnostic_events.jsonl \
  --out out/gb \
  --phase MVP-1 \
  --min-severity warn \
  --fail-on error

sarakura fc analyze \
  --metadata examples/fc/kitaqfc_build_metadata.json \
  --events examples/fc/kurosaki_diagnostic_events.jsonl \
  --out out/fc \
  --diagnostic-rule PPU_WRITE_OUTSIDE_VBLANK
```

## 未確認事項

この環境には `cargo` / `rustc` が無いため、実ビルド確認は未実施。Codex又は手元環境では以下を最初に実行すること。

```bash
cargo fmt --all -- --check
cargo test --workspace
```

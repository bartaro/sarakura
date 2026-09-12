# SARAKURA v0.5 changelog

## 目的

v0.5は、v0.4の安定版をベースに、KOKURA/KUROSAKI実行ログをCIと再テストへつなぐための運用機能を追加する。

## 追加内容

- `sarakura normalize-events` を追加。
  - `diagnostic_events.jsonl` をSARAKURA側で正規化し、同一 `summary_key` / stable key のイベントを集約する。
  - `count`, `first_seen`, `last_seen`, `severity` を統合する。
  - `--json-array` でfixture確認用のJSON配列も出力できる。
- `sarakura ci-summary` を追加。
  - `ai_diagnostics.json` からCI向けの小型サマリを生成する。
  - `--fail-on never|error|warn|info` と `--enforce` によりCI gateとして使える。
- `sarakura-core::normalize` を追加。
- `sarakura-core::ci` を追加。
- CLI smoke testを2件追加。
- `schemas/sarakura_ci_summary.schema.json` を追加。
- `schemas/sarakura_normalized_events.schema.json` を追加。

## 期待する使い方

```bash
sarakura normalize-events \
  --events out/kokura_diagnostic_events.jsonl \
  --out out/kokura_diagnostic_events.normalized.jsonl

sarakura gb analyze \
  --metadata out/kitaqgb_build_metadata.json \
  --events out/kokura_diagnostic_events.normalized.jsonl \
  --out out/sarakura_gb

sarakura ci-summary \
  --diagnostics out/sarakura_gb \
  --fail-on error \
  --enforce
```

## 完了条件

- v0.4の既存テストが維持される。
- GB catalog 48件、FC catalog 59件を維持する。
- `normalize-events` と `ci-summary` のCLI smoke testが通る。

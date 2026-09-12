# SARAKURA v0.2 修正メモ

作成日: 2026-05-17

## 目的

v0.1を、手元/Codex側で `cargo test --workspace` に掛けた時に出やすいビルドエラー・CLI挙動エラーを先回りで潰す。

## 主な修正

1. workspace versionを `0.2.0` へ更新。
2. `AnalyzeArgs` の `emit_report: bool` / `emit_repro_bundle: bool` を廃止し、Clapで扱いやすい `--no-report` / `--no-repro-bundle` に変更。
3. `compare` は `--before` / `--after` にディレクトリでも `ai_diagnostics.json` ファイルでも指定できるようにした。
4. `diagnostic_summary_limit` を実際に反映し、出力診断を重要度順に整列してからtruncateするようにした。
5. 重要度順は `error > warn > info`、次に `event_count`、次に `confidence` とした。
6. `DiagnosticEvent` loaderは本番JSONLに加えて、fixture用のJSON配列も受け付けるようにした。
7. `BuildMetadata::rom_hash()` を追加し、`rom.hash` / `rom.sha256` を `AiDiagnosticsDocument.rom.hash` に反映するようにした。
8. metadata correlationで `operation_id` / `intrinsic_id` / `vram_op_id` / `ppu_op_id` / `mapper_op_id` などのIDキーも認識するようにした。
9. PCが数値でmetadataに入っている場合も `0xXXXX` 形式に変換してsource mappingへ入れるようにした。
10. CLI integration testをGB/FC example成功パスまで拡張した。
11. READMEをv0.2のCLI仕様に合わせて更新した。

## 未確認事項

この作成環境には `cargo` / `rustc` がないため、実ビルド確認は未実施。手元またはCodex環境では次を実行する。

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo run -p sarakura-cli -- gb analyze \
  --metadata examples/gb/kitaqgb_build_metadata.json \
  --events examples/gb/kokura_diagnostic_events.jsonl \
  --out out/gb
```

## 次のv0.3候補

- `sarakura-schema` に `schemars` / `jsonschema` を導入した本格validation。
- `diagnostic_summary.json` に `suppressed_by_limit` を追加。
- `repair_prompt.md` をCodexにそのまま渡す形式へ強化。
- KOKURA/KUROSAKI emitterが出す追加fieldsに合わせたplatform-specific mapper。

# SARAKURA 工程表 2026-05-17

## 0. 目的

SARAKURA は、KITAQGB/KITAQFC の build metadata と、KOKURA/KUROSAKI の runtime diagnostic events を結合し、AI/Codex が修正可能な診断・証拠・修正計画・再現計画へ翻訳する Rust 製CLI / crate群である。

最終目標は、Mesen2 Lua scripting が得意とする手動・スクリプトデバッグを超えて、KITAQ系ツールチェーン専用の「診断 → 修正 → 再テスト → 回帰判定」自動ループを作ること。

---

## 1. 完了済み工程

| Version | 状態 | 主な成果 |
|---|---:|---|
| v0.1 | 完了 | Rust workspace、core/gb/fc/report/schema/cli の初期構成 |
| v0.2 | 完了 | CLI安定化、JSONL loader強化、summary limit、compare改善 |
| v0.3 | 完了 | catalog/phase/severity/filter、repair_prompt、snapshot/trace参照 |
| v0.4 | 完了 | retest_plan、inspect-events、inspect-metadata、emitter-check |
| v0.5 | 完了 | normalize-events、ci-summary、CI用fail-on判定 |
| v0.6 | 修正済み | coverage、pack-plan、diagnostic-pack filter |
| v0.6.1 | 完了 | v0.6のRunSummary初期化漏れ修正 |
| v0.7 | 完了 | repair_plan.json/md、repair target単位の修正順序化 |
| v0.8 | 完了 | automation_plan.json/md、KOKURA/KUROSAKI自動操作計画 |
| v0.9 | 修正済み | baseline-delta、new/resolved/regression/improvement判定 |
| v0.9.1 | 現在 | baseline.rsの参照型推論エラー修正 |

---

## 2. 直近工程 v0.10 - v0.12

### v0.10: 実diagnostic_events統合強化

目的：KOKURA/KUROSAKIの実ログをそのままSARAKURAへ入れても破綻しないようにする。

タスク：

- `event_type` alias table の導入
- KOKURA/KUROSAKI emitter version の互換性判定
- unknown event の分類改善
- severity正規化ルールの拡張
- frame/scanline/dot/cycle範囲の妥当性検査
- `coverage` に推奨emitter改修タスクを出力

完了条件：

- 実KOKURA/KUROSAKI JSONLを `normalize-events -> analyze -> coverage` に流せる
- unknown event が出た時に、catalogへ足すべきfield候補が出る

### v0.11: repro bundle v2

目的：AI/Codex/CIへ渡す再現資料を標準化する。

タスク：

- `repro manifest v2` 追加
- input ROM/hash、metadata/hash、events/hash、snapshot、trace、automation plan の参照を統一
- bundle integrity check コマンド追加
- `sarakura repro-inspect` コマンド追加
- SARAKURA出力一式の相対パス安定化

完了条件：

- bundle単体で、診断・修正計画・再テスト条件・自動操作コマンドを復元できる

### v0.12: AI修正プロンプト強化

目的：Codexへ渡す指示をより安全・具体的にする。

タスク：

- `codex_task_plan.md` 生成
- 修正禁止事項、触ってよい候補ファイル、優先順位、再テストコマンドを明記
- target_candidates を evidence_chain に基づいて順位付け
- confidence別の修正方針分岐

完了条件：

- `repair_prompt.md` よりも実装指示に特化した `codex_task_plan.md` が出る

---

## 3. 中期工程 v0.13 - v0.20

### v0.13: SARAKURA schema strict validation

- JSON Schemaを厳密化
- `validate --strict` 追加
- metadata/events/ai_diagnostics/retest/repair/automation/baseline/repro を横断検査

### v0.14: HTML report v2

- 診断別カードUI
- evidence chain timeline
- repair target grouping
- before/after delta view
- unknown event coverage view

### v0.15: KOKURA/KUROSAKI automation handoff

- automation_planからKOKURA/KUROSAKI CLIコマンド列を `.cmd` / `.sh` で出力
- Python bindings用サンプルスクリプト生成
- `break-on-diagnostic` 前提の再現スクリプト生成

### v0.16: Baseline policy file

- `sarakura_policy.toml` 追加
- project別の許容診断、fail-on-new、fail-on-regression、診断packを定義
- CI設定をコマンドラインからpolicy中心へ移行

### v0.17: Performance and streaming

- 巨大JSONLを全読みせずstream aggregate
- memory使用量削減
- events 100万件級のnormalize/coverage/analyzeに対応

### v0.18: SARAKURA library embedding

- `sarakura-core` public API整理
- KOKURA/KUROSAKIからcrateとして直接呼べるAPI安定化
- CLIとlibraryで出力差が出ないようにする

### v0.19: Rule authoring tools

- `sarakura rule-new`
- `sarakura rule-check`
- catalog ruleのfixture自動生成

### v0.20: Public preview hardening

- README/CLI docs整備
- examples拡充
- GitHub Actions workflow例
- MIT LICENSE / NOTICE / CONTRIBUTING整備

---

## 4. v1.0到達条件

v1.0は、以下を満たした時点で切る。

1. GB 48件、FC 59件のcatalogが安定している。
2. KOKURA/KUROSAKI emitter出力とSARAKURA入力schemaが一致している。
3. `analyze -> repair-plan -> automation-plan -> retest-plan -> baseline-delta` が一通り通る。
4. CIで `fail-on-new` / `fail-on-regression` が使える。
5. repro bundle単体でAI修正に必要な資料を渡せる。
6. Mesen2 Lua scriptingと比較して、AI修正・CI再現・診断自動化の面で明確な優位を説明できる。

---

## 5. KOKURA/KUROSAKI連携側の並行工程

### KOKURA

- Python bindings強化
- memory callback / input callback
- `break-on-diagnostic`
- GB/GBC向け diagnostic_events.jsonl emitter 実ログ強化
- SARAKURA automation_plan 実行アダプタ

### KUROSAKI

- Automation API / Python bindings強化
- run-until-pc / run-until-event / run-until-diagnostic
- memory/input callback
- DMC/PPU/mapper/FDS/VRC系イベントのJSONL出力
- SARAKURA repro bundleからの再現実行

---

## 6. 優先順位

最優先：

1. v0.10 実ログ統合強化
2. v0.11 repro bundle v2
3. v0.12 Codex task plan
4. KOKURA/KUROSAKI `break-on-diagnostic` と Python bindings連携

次優先：

5. HTML report v2
6. policy file
7. streaming aggregate
8. rule authoring tools

---

## 7. 現在の推奨次タスク

v0.9.1のビルドが通ったら、次は **v0.10 実diagnostic_events統合強化** に進む。

特に、KOKURA/KUROSAKI側の実emitterから出たJSONLを使い、以下を確認する。

```bash
sarakura normalize-events --events kokura_diagnostic_events.jsonl --out normalized.jsonl
sarakura coverage gb --events normalized.jsonl --out coverage.json
sarakura gb analyze --metadata kitaqgb_build_metadata.json --events normalized.jsonl --out sarakura_gb --diagnostic-pack core
sarakura automation-plan --diagnostics sarakura_gb --tool kokura --out sarakura_gb/automation_plan.json --markdown sarakura_gb/automation_plan.md
sarakura baseline-delta --baseline baseline_sarakura_gb --current sarakura_gb --out delta --markdown delta --fail-on-new error --fail-on-regression error
```

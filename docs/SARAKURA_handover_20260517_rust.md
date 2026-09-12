# SARAKURA 引継ぎ資料 Rust実装方針版

作成日：2026年5月17日  
対象：KITAQGB / KOKURA、KITAQFC / KUROSAKI、PicoWeave、SARAKURA、診断翻訳層、Codex実装仕様書改訂  
目的：SARAKURAを最初からRustで実装する方針を明確化し、次チャット・Codex・実装作業へ引き継ぐ。

---

## 1. 現時点の結論

PicoWeave と KITAQGB/KOKURA・KITAQFC/KUROSAKI 向け診断翻訳層は、分けて扱う。

さらに、SARAKURA は **Pythonではなく、最初からRustで実装する**。

### 1.1 PicoWeave

PicoWeave は、Pico / IoT / 組込み風DSL向けの参照実装として扱う。

役割：

- Pico / IoT 相当の小規模DSLを対象にする
- build_metadata、sim_ir、diagnostic_events、ai_diagnostics を生成する
- AI修正可能な診断翻訳の最小参照実装として使う
- 教材、論文、デモ、OSS普及用に使う
- KITAQGB/KOKURAやKITAQFC/KUROSAKIの中へ直接混ぜない

### 1.2 SARAKURA

SARAKURA は、KITAQGB/KOKURA および KITAQFC/KUROSAKI 専用の診断翻訳基盤として新規に作る。

仮の意味：

> SARAKURA = Source-Aware Runtime Analysis for KITAQ Unified Repair Assistance

日本語では、

> KITAQ系ツールチェーン向けソース認識型ランタイム診断翻訳基盤

と説明できる。

SARAKURA は、KITAQGB/KOKURA と KITAQFC/KUROSAKI の共通診断翻訳層であり、PicoWeave の plugin/adapters ではない。

---

## 2. SARAKURAをRustで書く理由

### 2.1 KOKURA / KUROSAKIとの親和性

KOKURA と KUROSAKI は Rust workspace として進められているため、SARAKURAもRustで作る方が自然である。

メリット：

- KOKURA/KUROSAKI本体とのコード共有がしやすい
- 将来的にRust crateとして直接組み込める
- CLIとしても単体バイナリ配布しやすい
- 速度面で大規模diagnostic_events.jsonlを処理しやすい
- 型安全にschemaと診断モデルを定義できる
- serdeによるJSON/JSONL処理が安定している
- HTML report、repro bundle、validationもRustで完結できる

### 2.2 Python案の撤回

前回資料では「最初はPython CLIが速い。将来的にRust crate化」としていたが、この方針は撤回する。

理由：

- SARAKURAはPicoWeaveとは異なり、GB/NESエミュレータ診断と密接に結び付く
- KOKURA/KUROSAKIと同じRust資産を使える方が長期的に有利
- 大量イベント処理・集約・HTML生成・schema validationを高速に処理したい
- Codexへ渡す実装仕様もRust workspace前提の方が明確

---

## 3. なぜ PicoWeave と SARAKURA を分けるのか

PicoWeave の plugin/adapters 構造を使って KITAQGB/KOKURA や KITAQFC/KUROSAKI へ拡張する案もあったが、最終的には分離した方がよい。

理由：

1. PicoWeave は Pico / IoT 向け参照実装としてシンプルに保てる。
2. KITAQGB/KOKURA と KITAQFC/KUROSAKI は、診断対象がGB/NES特有であり、Pico系より遥かに複雑である。
3. KOKURA/KUROSAKI はエミュレータ・解析・再現実行・プロファイラ・ROM診断を含むため、PicoWeaveの抽象に無理に合わせると不自然になる。
4. SARAKURA を専用ツールとして置くことで、GB/NES向けの診断翻訳・AI修正支援・HTMLレポート・repro bundle を独立して強化できる。
5. Rustで作ることで、KOKURA/KUROSAKIと同じ技術基盤に乗せられる。

---

## 4. SARAKURA の基本構成

### 4.1 全体フロー

```text
KITAQGB / KITAQFC
  ↓
build metadata
  ↓
KOKURA / KUROSAKI
  ↓
runtime diagnostic events
  ↓
SARAKURA
  ↓
diagnostic summary
  ↓
AI-repairable diagnostics
  ↓
HTML report / repair prompt / repro bundle
  ↓
AI or human repair
  ↓
KITAQGB / KITAQFC で再ビルド
```

### 4.2 SARAKURA の役割

SARAKURA は、以下を担当する。

- KITAQGB/KITAQFC が出す build metadata を読む
- KOKURA/KUROSAKI が出す diagnostic_events.jsonl を読む
- 同種イベントを集約・圧縮する
- PC / bank / address / symbol / function / source line / asset / intrinsic / queue 情報を対応付ける
- evidence_chain を生成する
- repair_target を生成する
- target_candidates を生成する
- safe_patch_hint を生成する
- confidence を付ける
- retest_condition を生成する
- ai_diagnostics.json を出す
- report.html を出す
- before/after report を出す
- repro bundle を出す

---

## 5. PicoWeave と SARAKURA の責任分担

| 項目 | PicoWeave | SARAKURA |
|---|---|---|
| 対象 | Pico / IoT / 組込み風DSL | Game Boy / GBC / NES / Famicom |
| 入力 | .pico DSL、sim_ir、Pico風metadata | KITAQGB/KITAQFC metadata、KOKURA/KUROSAKI events |
| 主な目的 | 診断翻訳層の分かりやすい参照実装 | KITAQGB/KOKURA・KITAQFC/KUROSAKIの実用診断翻訳 |
| 実行環境 | 仮想Picoボード | GB/GBC/NESエミュレータ |
| 実装言語 | Pythonでも可 | Rustで固定 |
| ライセンス方針 | MIT予定 | MIT予定 |
| 位置付け | 教材・論文・デモ | 実用・Codex・AIデバッグ基盤 |

---

## 6. SARAKURA Rust workspace 構成案

### 6.1 推奨リポジトリ構成

```text
sarakura/
  Cargo.toml
  README.md
  LICENSE
  crates/
    sarakura-core/
      Cargo.toml
      src/
        lib.rs
        model.rs
        aggregate.rs
        evidence.rs
        candidates.rs
        confidence.rs
        diagnostics.rs
        output.rs
    sarakura-gb/
      Cargo.toml
      src/
        lib.rs
        metadata.rs
        events.rs
        mapper.rs
        repair_hints.rs
        diagnostics_gb.rs
    sarakura-fc/
      Cargo.toml
      src/
        lib.rs
        metadata.rs
        events.rs
        mapper.rs
        repair_hints.rs
        diagnostics_fc.rs
    sarakura-report/
      Cargo.toml
      src/
        lib.rs
        html.rs
        markdown.rs
        compare.rs
    sarakura-schema/
      Cargo.toml
      src/
        lib.rs
        validate.rs
    sarakura-cli/
      Cargo.toml
      src/
        main.rs
        cmd_gb.rs
        cmd_fc.rs
        cmd_validate.rs
        cmd_compare.rs
        cmd_repro.rs
  docs/
    SARAKURA_SPEC.md
    AI_DIAGNOSTIC_SCHEMA.md
    GB_DIAGNOSTICS.md
    FC_DIAGNOSTICS.md
    CLI.md
    REPORT_FORMAT.md
  schemas/
    sarakura_build_metadata.schema.json
    sarakura_diagnostic_event.schema.json
    sarakura_ai_diagnostics.schema.json
    sarakura_summary.schema.json
  examples/
    gb/
    fc/
  tests/
    fixtures/
```

### 6.2 推奨crate責務

| crate | 役割 |
|---|---|
| sarakura-core | 共通モデル、集約、evidence_chain、target_candidates、confidence |
| sarakura-gb | KITAQGB/KOKURA用metadata/event parser、GB固有mapper |
| sarakura-fc | KITAQFC/KUROSAKI用metadata/event parser、FC/NES固有mapper |
| sarakura-report | HTML/Markdown/before-after report生成 |
| sarakura-schema | JSON Schema validation |
| sarakura-cli | CLI入口 |

### 6.3 推奨依存crate

候補：

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_jsonlines = "0.7"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "1"
camino = "1"
indexmap = "2"
schemars = "0.8"
jsonschema = "0.18"
askama = "0.12" # または minijinja
zip = "0.6"     # repro bundle用
time = "0.3"
```

テスト候補：

```toml
assert_cmd = "2"
predicates = "3"
tempfile = "3"
insta = "1"
pretty_assertions = "1"
```

---

## 7. SARAKURA CLI 案

### 7.1 GB側

```bash
sarakura gb analyze   --metadata out/kitaqgb_build_metadata.json   --events out/kokura_diagnostic_events.jsonl   --out out/sarakura_gb
```

出力：

```text
out/sarakura_gb/
  diagnostic_summary.json
  ai_diagnostics.json
  report.html
  repair_prompt.md
  repro_bundle.zip
```

### 7.2 FC/NES側

```bash
sarakura fc analyze   --metadata out/kitaqfc_build_metadata.json   --events out/kurosaki_diagnostic_events.jsonl   --out out/sarakura_fc
```

出力：

```text
out/sarakura_fc/
  diagnostic_summary.json
  ai_diagnostics.json
  report.html
  repair_prompt.md
  repro_bundle.zip
```

### 7.3 Schema validation

```bash
sarakura validate out/sarakura_gb/ai_diagnostics.json
sarakura validate out/sarakura_fc/ai_diagnostics.json
```

### 7.4 Before / After 比較

```bash
sarakura compare   --before out/before_sarakura   --after out/after_sarakura   --out out/comparison_report
```

---

## 8. GB側：KITAQGB / KOKURA の必要出力

### 8.1 KITAQGB 側

KITAQGB は、少なくとも以下を出力する。

```text
kitaqgb_build_metadata.json
```

含めるべき情報：

- source_file
- source_line
- function_id
- function_name
- rom_bank
- pc_range
- symbol_name
- intrinsic_id
- intrinsic_kind
- operation_kind
- requires_vblank
- vram_op_id
- oam_op_id
- dma_op_id
- apu_update_id
- bank_switch_id
- farcall_id
- asset_id
- tilemap_id
- sprite_id
- bgm_id
- section
- wram_bank
- hram_usage
- queue_id
- ABI情報
- SP/register期待値
- build warning対応

### 8.2 KOKURA 側

KOKURA は、少なくとも以下を出力する。

```text
kokura_diagnostic_events.jsonl
```

1行1イベント。巨大JSON配列にはしない。

含めるべき情報：

- event_id
- event_type
- severity
- frame
- scanline
- dot/cycle
- pc
- rom_bank
- addr
- access_kind
- ppu_mode
- lcdc/stat state
- dma_state
- apu_state
- function_hint
- symbol_hint
- count
- first_seen
- last_seen
- snapshot_ref
- trace_window_ref

---

## 9. FC側：KITAQFC / KUROSAKI の必要出力

### 9.1 KITAQFC 側

KITAQFC は、少なくとも以下を出力する。

```text
kitaqfc_build_metadata.json
```

含めるべき情報：

- source_file
- source_line
- function_id
- function_name
- prg_bank
- pc_range
- symbol_name
- intrinsic_id
- operation_kind
- ppu_op_id
- palette_op_id
- oam_dma_op_id
- nmi_handler_id
- irq_handler_id
- dmc_read_risk_id
- mapper_id
- mapper_op_id
- fds_op_id
- vrc6_audio_op_id
- vrc7_audio_op_id
- section
- zero_page_usage
- stack_usage
- build warning対応

### 9.2 KUROSAKI 側

KUROSAKI は、少なくとも以下を出力する。

```text
kurosaki_diagnostic_events.jsonl
```

含めるべき情報：

- event_id
- event_type
- severity
- frame
- scanline
- dot/cycle
- pc
- prg_bank
- addr
- access_kind
- ppu_state
- nmi_state
- irq_state
- dma_state
- mapper_state
- apu_state
- dmc_state
- fds_state
- vrc_state
- count
- first_seen
- last_seen
- snapshot_ref
- trace_window_ref

---

## 10. SARAKURA の全想定診断セット

直近で作成済みの v3 詳細仕様書では、以下の全想定診断セットを入れた。

### 10.1 KITAQGB / KOKURA 側

診断数：48件

代表カテゴリ：

- VRAM / PPU access
- OAM / sprite / DMA
- HDMA / GDMA
- VBlank / WaitVBlank / LCDC / STAT
- bank switch / farcall
- APU / audio timing
- interrupt / stack / ABI
- asset / tilemap / sprite metadata mismatch
- profiler / culprit ranking
- replay / snapshot / trace link

### 10.2 KITAQFC / KUROSAKI 側

診断数：59件

代表カテゴリ：

- PPU write / palette / scroll
- OAM DMA / sprite overflow
- NMI / IRQ / DMC DMA
- mapper / MMC3 IRQ
- FDS disk / FDS wave
- VRC6 / VRC7 audio
- bank switch / PRG/CHR
- stack / zero page / ABI
- controller read / timing conflict
- replay / snapshot / trace link

### 10.3 注意

v3仕様書は、PicoWeave前提ではなく、SARAKURA Rust workspace前提に書き直す必要がある。内容は活かしつつ、以下を変更する。

- PicoWeave adapter という表現を削除
- SARAKURA CLI として再定義
- Python実装案を削除
- Rust workspace / crates 構成を明記
- `picoweave` 名前を使わない
- `sarakura_ai_diagnostics.json` という名前を使うか検討
- GB/FC共通coreとGB/FC専用mapperをSARAKURA内部に置く
- KITAQGB/KOKURA、KITAQFC/KUROSAKI連携をSARAKURAの主目的にする

---

## 11. Codexへ次に依頼する作業

### 11.1 最優先

前回作成した以下の仕様書を、SARAKURA Rust前提で書き直す。

- `kitaqgb_kokura_ai_diagnostic_bridge_codex_spec_v3_all_diagnostics_20260516.md`
- `kitaqfc_kurosaki_ai_diagnostic_bridge_codex_spec_v3_all_diagnostics_20260516.md`

改訂後のファイル名案：

- `sarakura_gb_kitaqgb_kokura_codex_spec_v1_rust_20260517.md`
- `sarakura_fc_kitaqfc_kurosaki_codex_spec_v1_rust_20260517.md`
- `sarakura_codex_specs_v1_rust_20260517.zip`

### 11.2 改訂仕様書に必ず入れること

1. SARAKURA の定義
2. PicoWeave との分離方針
3. Rust workspace構成
4. crate責務
5. GB CLI
6. FC CLI
7. 入出力schema
8. 巨大JSON回避方針
9. GB 48診断
10. FC 59診断
11. 各診断の検出条件
12. 必要メタデータ
13. diagnostic_events.jsonl fields
14. ai_diagnostics fields
15. evidence_chain
16. target_candidates
17. repair_target
18. safe_patch_hint
19. confidence
20. retest_condition
21. HTML report
22. before/after report
23. repro bundle
24. schema validation
25. Codex実装タスク
26. Rust unit/integration tests
27. fixture構成
28. 完了条件

### 11.3 実装はまだ行わない

今回の直近タスクは、まず SARAKURA Rust前提の詳細仕様書を書き直すこと。  
実装は次段階。

---

## 12. OSS / 特許方針

### 12.1 直近のユーザー判断

ユーザーは、特許出願せず、全てMITライセンスで公開して考え方を普及させる方が利益が大きいのではないかと考えている。

この判断は妥当性がある。

理由：

- AI/ML領域では、特許独占よりOSS普及で影響力を取った事例が多い
- scikit-learn、PyTorch、TensorFlow、LangChain等のように、OSSと論文・ドキュメント・コミュニティで標準化する道がある
- KITAQGB/KOKURA、KITAQFC/KUROSAKI、PicoWeave、SARAKURAは、特許権より普及・教育・PoC・サポート・クラウドで収益化しやすい可能性がある

### 12.2 推奨ライセンス

現時点の推奨：

| プロジェクト | ライセンス |
|---|---|
| KITAQGB | MIT |
| KOKURA | MIT |
| KITAQFC | MIT |
| KUROSAKI | MIT |
| PicoWeave | MIT |
| SARAKURA | MIT |

ただし、商標は別途検討してよい。

---

## 13. 次チャット開始プロンプト案

次チャットでは、以下を貼ると続けやすい。

```text
SARAKURA関連作業の続きです。

方針として、PicoWeaveはPico/IoT向け参照実装、SARAKURAはKITAQGB/KOKURAおよびKITAQFC/KUROSAKI向けの専用診断翻訳基盤として分けます。

SARAKURAはPythonではなく、最初からRust workspace / Rust CLI / Rust cratesとして実装します。

SARAKURAは、KITAQGB/KITAQFCが出すbuild metadataと、KOKURA/KUROSAKIが出すruntime diagnostic eventsを読み、diagnostic summary、evidence_chain、target_candidates、repair_target、safe_patch_hint、confidence、retest_condition、HTML report、before/after report、repro bundleを生成する外部CLI/共通基盤です。

前回までに作ったv3詳細仕様書はPicoWeave plugin/adapters前提やPython案が残っているため、SARAKURA Rust前提に書き直してください。

作成してほしいファイルは以下です。

1. sarakura_gb_kitaqgb_kokura_codex_spec_v1_rust_20260517.md
2. sarakura_fc_kitaqfc_kurosaki_codex_spec_v1_rust_20260517.md
3. 上記2ファイルをまとめたzip

GB側は48件、FC側は59件の全想定診断セットを維持し、各診断について、検出条件、必要メタデータ、diagnostic_events.jsonl fields、ai_diagnostics fields、evidence_chain、target_candidates、repair_target、safe_patch_hint、confidence、retest_condition、テストfixture、Codex実装タスクまで記載してください。

また、Rust workspace構成、crate分割、推奨依存crate、Rust unit/integration tests、CLI仕様も明記してください。
```

---

## 14. 現時点の最終整理

- PicoWeave は Pico / IoT の参照実装。
- SARAKURA は KITAQGB/KOKURA・KITAQFC/KUROSAKI 専用診断翻訳基盤。
- SARAKURA は Python ではなく最初から Rust で書く。
- PicoWeave plugin/adapters でKITAQGB/KOKURAへ拡張する案は撤回。
- SARAKURAを別プロジェクトとして作る。
- 既存v3仕様書はSARAKURA Rust前提に書き直す。
- ライセンスは全てMITが現時点の有力方針。
- 特許出願は行わず、防衛公開・OSS普及・arXiv・技術ブログ・公式ドキュメントで広める方向が有力。
- 次の作業は、SARAKURA Rust用Codex詳細仕様書の再作成。

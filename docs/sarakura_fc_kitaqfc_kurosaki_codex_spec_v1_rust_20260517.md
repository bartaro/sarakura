# SARAKURA FC / KITAQFC + KUROSAKI Codex Spec v1 Rust

作成日: 2026-05-17  
対象: KITAQFC / KUROSAKI / NES / Famicom  
文書種別: Codex実装仕様書 / SARAKURA Rust v1  
診断件数: 59件  

---

## 1. 目的

本書は、SARAKURAを **Pythonではなく最初からRust workspace / Rust CLI / Rust crates** として実装するための、Codex向け詳細仕様書である。

SARAKURAは、PicoWeaveとは別プロジェクトである。PicoWeaveはPico/IoT向け参照実装として独立させ、SARAKURAはKITAQ系ツールチェーン専用の診断翻訳基盤として扱う。

SARAKURAは、build metadata と runtime diagnostic events を読み、以下を生成する。

- `diagnostic_summary.json`
- `ai_diagnostics.json`
- `evidence_chain`
- `target_candidates`
- `repair_target`
- `safe_patch_hint`
- `confidence`
- `retest_condition`
- `report.html`
- before/after report
- `repro_bundle.zip`

## 2. SARAKURA の定義と境界

SARAKURA = Source-Aware Runtime Analysis for KITAQ Unified Repair Assistance。

日本語では「KITAQ系ツールチェーン向けソース認識型ランタイム診断翻訳基盤」とする。

### 2.1 役割

```text
KITAQGB / KITAQFC build metadata
          +
KOKURA / KUROSAKI runtime diagnostic events
          ↓
SARAKURA Rust CLI
          ↓
diagnostic summary / AI diagnostics / HTML report / repro bundle
```

### 2.2 非目標

- SARAKURA内部でROMを実行すること。
- SARAKURA内部でKITAQGB/KITAQFCのコンパイルを行うこと。
- Python実装を先行させること。
- PicoWeave側の拡張機構としてGB/FC診断を扱うこと。
- safe_patch_hintを無条件自動パッチとして適用すること。

## 3. 入力仕様: KITAQFC / KUROSAKI

### 3.1 KITAQFC build metadata

ファイル名の既定値は `kitaqfc_build_metadata.json`。

必須・推奨フィールド：

- `source_file`
- `source_line`
- `function_id`
- `function_name`
- `prg_bank`
- `pc_range`
- `symbol_name`
- `intrinsic_id`
- `operation_kind`
- `ppu_op_id`
- `palette_op_id`
- `oam_dma_op_id`
- `nmi_handler_id`
- `irq_handler_id`
- `dmc_read_risk_id`
- `mapper_id`
- `mapper_op_id`
- `fds_op_id`
- `vrc6_audio_op_id`
- `vrc7_audio_op_id`
- `section`
- `zero_page_usage`
- `stack_usage`
- `build_warning_id`

最小例：

```json
{
  "schema_version": "kitaqfc.build_metadata.v1",
  "target": "nes",
  "mapper": { "mapper_id": 4, "name": "MMC3" },
  "functions": [
    {
      "function_id": "fn_nmi_update",
      "function_name": "nmi_update",
      "source_file": "src/main.c",
      "source_line": 88,
      "prg_bank": 0,
      "pc_range": { "start": 32768, "end": 32920 },
      "symbol_name": "_nmi_update"
    }
  ],
  "operations": []
}
```

### 3.2 KUROSAKI diagnostic events

ファイル名の既定値は `kurosaki_diagnostic_events.jsonl`。1行1イベント。巨大JSON配列は禁止する。

必須・推奨フィールド：

- `event_id`
- `event_type`
- `severity`
- `frame`
- `scanline`
- `dot`
- `cycle`
- `pc`
- `prg_bank`
- `addr`
- `access_kind`
- `ppu_state`
- `nmi_state`
- `irq_state`
- `dma_state`
- `mapper_state`
- `apu_state`
- `dmc_state`
- `fds_state`
- `vrc_state`
- `count`
- `first_seen`
- `last_seen`
- `snapshot_ref`
- `trace_window_ref`

最小例：

```jsonl
{"event_id":"ev1","event_type":"fc.ppu.write.unsafe","severity":"error","frame":12,"scanline":120,"dot":40,"pc":32820,"prg_bank":0,"addr":8198,"access_kind":"write","ppu_state":{"rendering":true},"count":1,"first_seen":12,"last_seen":12}
```

## 4. Rust workspace 構成

SARAKURA は最初から Rust workspace として実装する。Python CLIおよびPicoWeave側の拡張機構は本仕様の実装対象外とする。

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
        repro.rs
    sarakura-gb/
      Cargo.toml
      src/
        lib.rs
        metadata.rs
        events.rs
        mapper.rs
        repair_hints.rs
        diagnostics_gb.rs
        fixtures.rs
    sarakura-fc/
      Cargo.toml
      src/
        lib.rs
        metadata.rs
        events.rs
        mapper.rs
        repair_hints.rs
        diagnostics_fc.rs
        fixtures.rs
    sarakura-report/
      Cargo.toml
      src/
        lib.rs
        html.rs
        markdown.rs
        compare.rs
        template.rs
    sarakura-schema/
      Cargo.toml
      src/
        lib.rs
        validate.rs
        schema_export.rs
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
      gb/
      fc/
```

### 4.1 crate 責務

| crate | 責務 |
|---|---|
| `sarakura-core` | 共通データモデル、JSONL streaming reader、集約、重複圧縮、evidence_chain、target_candidates、confidence、retest_condition、repro bundle manifest |
| `sarakura-gb` | KITAQGB build metadata parser、KOKURA diagnostic event parser、GB/GBC固有診断48件、GB固有repair hint、GB fixture生成 |
| `sarakura-fc` | KITAQFC build metadata parser、KUROSAKI diagnostic event parser、NES/Famicom固有診断59件、mapper/FDS/VRC系repair hint、FC fixture生成 |
| `sarakura-report` | HTML report、Markdown report、repair_prompt.md、before/after report、diff table生成 |
| `sarakura-schema` | `schemars` によるschema生成、`jsonschema` による入力/出力validation |
| `sarakura-cli` | `sarakura gb analyze` / `sarakura fc analyze` / `validate` / `compare` / `repro pack` の入口 |

### 4.2 推奨依存crate

```toml
[workspace.dependencies]
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
askama = "0.12"      # minijinja でも可。HTML report用。
zip = "0.6"          # repro bundle用。新規採用時は最新版に追随してよい。
time = { version = "0.3", features = ["formatting", "parsing"] }
tracing = "0.1"
tracing-subscriber = "0.3"

[workspace.dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
insta = "1"
pretty_assertions = "1"
```

### 4.3 型定義方針

- 入力JSONは `serde(deny_unknown_fields)` を原則にしない。KITAQGB/KITAQFC/KOKURA/KUROSAKI側の拡張に耐えるため、未知フィールドは `extra: BTreeMap<String, Value>` に逃がす。
- SARAKURA内部の正規化後モデルは強い型を使う。
- `pc`、`addr`、`rom_bank`、`prg_bank`、`frame`、`scanline`、`dot` は比較・ソート・集約キーになるため、文字列ではなく数値で保持する。
- 表示用には `bank:addr`、`frame:scanline.dot` 形式へ変換する。
- JSONLは全件メモリ展開せず、streamingで読み、同種イベントを `DiagnosticBucket` に集約する。

## 5. 共通出力schema

### 5.1 `diagnostic_summary.json`

```json
{
  "schema_version": "sarakura.summary.v1",
  "target": "gb-or-fc",
  "toolchain": "KITAQGB-or-KITAQFC",
  "runtime": "KOKURA-or-KUROSAKI",
  "generated_at": "2026-05-17T00:00:00+09:00",
  "input": {
    "metadata_path": "out/build_metadata.json",
    "events_path": "out/diagnostic_events.jsonl",
    "metadata_hash": "sha256:...",
    "events_hash": "sha256:..."
  },
  "counts": {
    "events_total": 0,
    "diagnostics_total": 0,
    "critical": 0,
    "error": 0,
    "warning": 0,
    "info": 0
  },
  "top_diagnostics": []
}
```

### 5.2 `ai_diagnostics.json`

`ai_diagnostics.json` はAI修正に直接渡す主出力であり、下記フィールドを必須とする。

```json
{
  "schema_version": "sarakura.ai_diagnostics.v1",
  "target": "gb-or-fc",
  "diagnostics": [
    {
      "diagnostic_id": "GB001-or-FC001",
      "title": "human readable title",
      "category": "vram-or-ppu-or-mapper",
      "severity": "error",
      "summary": "one paragraph summary",
      "detected_condition": "normalized detection condition",
      "event_refs": ["event_id"],
      "metadata_refs": ["function_id", "intrinsic_id", "asset_id"],
      "evidence_chain": [],
      "target_candidates": [],
      "repair_target": {},
      "safe_patch_hint": {},
      "confidence": { "score": 0.0, "basis": [], "downgrade_reasons": [] },
      "retest_condition": {},
      "fixture": {},
      "codex_tasks": []
    }
  ]
}
```

### 5.3 `evidence_chain` 要素

```json
{
  "kind": "event|metadata|trace|snapshot|static_mapping|aggregation",
  "message": "why this evidence matters",
  "ref": "event_id-or-metadata-id-or-trace-window",
  "location": {
    "source_file": "src/main.c",
    "source_line": 123,
    "function_name": "draw_board",
    "bank": 2,
    "pc": 49152
  }
}
```

### 5.4 `target_candidates` 要素

```json
{
  "rank": 1,
  "kind": "function|intrinsic|asset|queue|isr|mapper_op|audio_op",
  "id": "function_id-or-asset_id",
  "name": "draw_board",
  "source_file": "src/main.c",
  "source_line": 123,
  "reason": "mapped by pc_range and repeated event concentration",
  "confidence": 0.92
}
```

### 5.5 `repair_target`

```json
{
  "primary_kind": "source_line|intrinsic_call|runtime_queue|asset_metadata|mapper_setup",
  "source_file": "src/main.c",
  "source_line": 123,
  "function_name": "draw_board",
  "symbol_name": "_draw_board",
  "operation_kind": "vram_write",
  "preferred_edit_scope": "smallest-safe-change"
}
```

### 5.6 `safe_patch_hint`

`safe_patch_hint` は自動パッチそのものではなく、Codexまたは人間が安全に修正するための制約つきヒントである。

```json
{
  "strategy": "move-to-vblank-queue|guard-with-wait|restore-bank|split-transfer|add-shadow-buffer|fix-metadata",
  "allowed_change_scope": ["source", "metadata", "runtime-helper"],
  "must_not_change": ["public ABI", "ROM banking layout unless explicitly required"],
  "example_patch_shape": "pseudo diff or prose",
  "risk_notes": []
}
```

### 5.7 `confidence`

- `0.90 - 1.00`: PC範囲、symbol、metadata id、trace window が一致。修正対象をほぼ特定。
- `0.70 - 0.89`: PC範囲とイベント種別は一致。候補が複数ある。
- `0.50 - 0.69`: runtime eventは明確だがmetadataが不足。
- `0.00 - 0.49`: 症状のみ。reportには出すが、自動修正候補からは外す。

### 5.8 `retest_condition`

```json
{
  "command": "rebuild-and-run command placeholder",
  "required_events_absent": ["same event_type at same pc"],
  "required_metrics": {
    "max_same_diagnostic_count": 0,
    "max_frame_time_overrun": 0
  },
  "minimum_replay_frames": 600,
  "snapshot_compare": true
}
```

## 6. Report / Repro bundle 仕様

### 6.1 HTML report

`report.html` は単一HTMLで生成する。外部CSS/JSに依存しない。

必須セクション：

1. Summary cards: target、input hash、diagnostic count、critical/error/warning数
2. Top culprit ranking: function / source line / asset / mapper / queueごとの集約
3. Diagnostics table: diagnostic_id、severity、confidence、repair_target、retest_condition
4. Evidence chain view: event -> PC/bank -> symbol/function -> source -> suggested repair
5. Timeline view: frame / scanline / dot でソートしたイベント列
6. Source target view: 修正候補のファイル・行番号一覧
7. Repro section: 再実行コマンド、必要ROM、metadata、events、snapshot、trace window

### 6.2 Before / After report

`compare` は2つのSARAKURA出力ディレクトリを比較し、下記を生成する。

```text
comparison_report/
  before_after_summary.json
  before_after_report.html
  resolved_diagnostics.json
  new_diagnostics.json
  remaining_diagnostics.json
```

比較キー：

- `diagnostic_id`
- `event_type`
- `pc` / `addr`
- `bank`
- `function_id`
- `asset_id` / `mapper_op_id` / `queue_id`

判定：

- `resolved`: beforeに存在しafterで消えた
- `improved`: countまたはseverityが下がった
- `unchanged`: 同一条件で残った
- `regressed`: countまたはseverityが上がった
- `new`: afterで新規発生

### 6.3 Repro bundle

`repro_bundle.zip` には、権利上同梱可能なものだけを入れる。ROMの同梱はオプションで、既定ではhashとパス参照に留める。

```text
repro_bundle.zip
  manifest.json
  metadata/build_metadata.json
  events/diagnostic_events.jsonl
  sarakura/diagnostic_summary.json
  sarakura/ai_diagnostics.json
  sarakura/report.html
  traces/*.jsonl
  snapshots/*.bin-or-json
  commands/retest.sh
  commands/retest.ps1
```

`manifest.json` 必須フィールド：

```json
{
  "schema_version": "sarakura.repro_bundle.v1",
  "target": "gb-or-fc",
  "created_at": "2026-05-17T00:00:00+09:00",
  "input_hashes": {},
  "tool_versions": {},
  "rom": { "included": false, "sha256": "..." },
  "files": []
}
```

## 7. Rust tests / fixture 方針

### 7.1 unit tests

- `sarakura-core::aggregate`:
  - JSONL streaming readerが巨大配列を要求しないこと
  - 同一event_type + pc + bank + operation_idを同一bucketに集約すること
  - first_seen / last_seen / count を正しく更新すること
- `sarakura-core::confidence`:
  - metadata完全一致で0.90以上になること
  - metadata不足時に0.70未満へ下がること
  - snapshot/trace欠落時にdowngrade_reasonsが入ること
- `sarakura-core::evidence`:
  - event -> pc_range -> function -> source_line の順で evidence_chain を構築すること
- `sarakura-report`:
  - HTML escapingを行うこと
  - reportが単一HTMLとして生成されること
- `sarakura-schema`:
  - schema exportとvalidationが成功すること

### 7.2 integration tests

- `sarakura gb analyze --metadata fixtures/gb/min/metadata.json --events fixtures/gb/min/events.jsonl --out tmp/out`
- `sarakura fc analyze --metadata fixtures/fc/min/metadata.json --events fixtures/fc/min/events.jsonl --out tmp/out`
- `sarakura validate tmp/out/ai_diagnostics.json`
- `sarakura compare --before fixtures/before --after fixtures/after --out tmp/compare`
- `sarakura repro pack --analysis tmp/out --out tmp/repro_bundle.zip`

### 7.3 snapshot tests

`insta` を使い、以下をsnapshot化する。

- `diagnostic_summary.json`
- `ai_diagnostics.json` の主要部分
- `repair_prompt.md`
- HTML reportの見出し・table構造

### 7.4 fixture naming

各診断に最低1 fixtureを置く。

```text
tests/fixtures/<target>/<diagnostic_id>/
  build_metadata.json
  diagnostic_events.jsonl
  expected_ai_diagnostics.json
  expected_summary.json
  README.md
```

## 8. FC CLI 仕様

### 8.1 analyze

```bash
sarakura fc analyze \
  --metadata out/kitaqfc_build_metadata.json \
  --events out/kurosaki_diagnostic_events.jsonl \
  --out out/sarakura_fc
```

Options:

| option | 必須 | 説明 |
|---|---:|---|
| `--metadata <path>` | yes | KITAQFCが出力したbuild metadata JSON |
| `--events <path>` | yes | KUROSAKIが出力したruntime diagnostic events JSONL |
| `--out <dir>` | yes | SARAKURA出力ディレクトリ |
| `--rom <path>` | no | repro bundleにhashだけ記録。`--include-rom`がない限り同梱しない |
| `--symbols <path>` | no | KITAQFC/KUROSAKI symbol map補助入力 |
| `--mapper <name-or-id>` | no | mapper推定を上書きする。MMC3/FDS/VRC等のfixtureで使う |
| `--trace-dir <dir>` | no | trace window参照先 |
| `--snapshot-dir <dir>` | no | snapshot参照先 |
| `--min-confidence <float>` | no | repair_promptへ出す最低confidence。既定0.50 |
| `--strict-schema` | no | unknown fieldsをwarningではなくerrorにする |

### 8.2 output

```text
out/sarakura_fc/
  diagnostic_summary.json
  ai_diagnostics.json
  report.html
  repair_prompt.md
  repro_bundle.zip
```

### 8.3 validate

```bash
sarakura validate out/sarakura_gb/ai_diagnostics.json
sarakura validate out/sarakura_fc/ai_diagnostics.json
```

### 8.4 compare

```bash
sarakura compare \
  --before out/before_sarakura \
  --after out/after_sarakura \
  --out out/comparison_report
```

### 8.5 repro pack

```bash
sarakura repro pack \
  --analysis out/sarakura_gb \
  --out out/repro_bundle.zip
```

FCの場合は `--analysis out/sarakura_fc` を指定する。
## 9. FC側 全想定診断セット 59件

以下の59件をSARAKURA FC v1の全診断セットとする。

## 10. 診断一覧サマリ

| ID | Category | Primary event_type | Severity | Title |
|---|---|---|---|---|
| FC001 | PPU write / palette / scroll | `fc.ppu.write.unsafe` | `error` | PPU write outside VBlank or forced blank |
| FC002 | PPU write / palette / scroll | `fc.palette.write.unsafe` | `error` | Palette write glitch risk |
| FC003 | PPU write / palette / scroll | `fc.scroll.sequence.broken` | `error` | Scroll write sequence broken |
| FC004 | PPU write / palette / scroll | `fc.ppuaddr.latch.desync` | `error` | PPUADDR latch desynchronization |
| FC005 | PPU write / palette / scroll | `fc.ppuctrl.nmi_toggle.unsafe` | `error` | PPUCTRL NMI bit changed unsafely |
| FC006 | PPU write / palette / scroll | `fc.ppumask.render_toggle.unsafe` | `error` | PPUMASK rendering toggle unsafe |
| FC007 | PPU write / palette / scroll | `fc.vram_queue.over_budget` | `warning` | VRAM update queue exceeds NMI budget |
| FC008 | PPU write / palette / scroll | `fc.attribute_table.mismatch` | `warning` | Attribute table metadata mismatch |
| FC009 | PPU write / palette / scroll | `fc.chr_bank.asset_mismatch` | `error` | CHR bank asset mismatch |
| FC010 | PPU write / palette / scroll | `fc.nametable.mirroring_mismatch` | `error` | Nametable mirroring mismatch |
| FC011 | PPU write / palette / scroll | `fc.split_scroll.timing_miss` | `warning` | Split scroll raster timing miss |
| FC012 | PPU write / palette / scroll | `fc.sprite0_hit.unstable` | `warning` | Sprite zero hit timing unstable |
| FC013 | OAM DMA / sprite overflow | `fc.oamdma.unsafe_window` | `error` | OAM DMA outside safe window |
| FC014 | OAM DMA / sprite overflow | `fc.oam_buffer.overflow` | `error` | OAM buffer overflow |
| FC015 | OAM DMA / sprite overflow | `fc.sprite.scanline.overflow` | `warning` | Sprite overflow on scanline |
| FC016 | OAM DMA / sprite overflow | `fc.oamdma.page_mismatch` | `error` | OAM DMA page crosses unexpected RAM page |
| FC017 | OAM DMA / sprite overflow | `fc.metasprite.coord_wrap` | `warning` | Metasprite coordinate wrap risk |
| FC018 | OAM DMA / sprite overflow | `fc.sprite.chr_bank_mismatch` | `error` | Sprite tile CHR bank mismatch |
| FC019 | OAM DMA / sprite overflow | `fc.oamdma.dmc_conflict` | `warning` | OAM DMA / DMC conflict risk |
| FC020 | NMI / IRQ / DMC / controller | `fc.nmi.frame_overrun` | `warning` | NMI handler frame overrun |
| FC021 | NMI / IRQ / DMC / controller | `fc.nmi.reentrant` | `error` | NMI reentrancy or NMI during update |
| FC022 | NMI / IRQ / DMC / controller | `fc.irq.ack_late` | `error` | IRQ acknowledge missing or late |
| FC023 | NMI / IRQ / DMC / controller | `fc.dmc.controller_conflict` | `warning` | DMC DMA controller read conflict |
| FC024 | NMI / IRQ / DMC / controller | `fc.controller.double_clock` | `error` | Controller double clock or misread |
| FC025 | NMI / IRQ / DMC / controller | `fc.nmi_main.queue_race` | `error` | NMI/main thread queue race |
| FC026 | NMI / IRQ / DMC / controller | `fc.irq.vector_bank_mismatch` | `error` | IRQ vector bank mismatch |
| FC027 | NMI / IRQ / DMC / controller | `fc.vblank_flag.stale` | `warning` | VBlank flag stale read |
| FC028 | NMI / IRQ / DMC / controller | `fc.apu.frame_counter_mismatch` | `warning` | APU frame counter timing mismatch |
| FC029 | mapper / MMC / bank switch | `fc.mapper.write.wrong_bank` | `error` | Mapper write goes to wrong bank/register |
| FC030 | mapper / MMC / bank switch | `fc.mmc1.serial.interrupted` | `error` | MMC1 serial write interrupted |
| FC031 | mapper / MMC / bank switch | `fc.mmc3.irq.timing_mismatch` | `error` | MMC3 scanline IRQ timing mismatch |
| FC032 | mapper / MMC / bank switch | `fc.mmc3.a12.false_trigger` | `warning` | MMC3 A12 filter false trigger |
| FC033 | mapper / MMC / bank switch | `fc.mmc5.exram.mode_mismatch` | `error` | MMC5 ExRAM mode mismatch |
| FC034 | mapper / MMC / bank switch | `fc.banked_call.no_trampoline` | `error` | Banked call without trampoline |
| FC035 | mapper / MMC / bank switch | `fc.prg_bank.restore_missing` | `error` | PRG bank restore missing |
| FC036 | mapper / MMC / bank switch | `fc.chr_bank.swap_during_render` | `error` | CHR bank swapped during rendering |
| FC037 | mapper / MMC / bank switch | `fc.mapper.shadow_desync` | `warning` | Mapper register shadow desync |
| FC038 | mapper / MMC / bank switch | `fc.mapper.unsupported_feature` | `error` | Unsupported mapper feature used |
| FC039 | FDS disk / FDS wave | `fc.fds.disk_side.mismatch` | `error` | FDS disk side state mismatch |
| FC040 | FDS disk / FDS wave | `fc.fds.read.timing_risk` | `warning` | FDS disk read timing risk |
| FC041 | FDS disk / FDS wave | `fc.fds.irq.unacked` | `error` | FDS IRQ transfer unacknowledged |
| FC042 | FDS disk / FDS wave | `fc.fds.wave.write_while_playing` | `error` | FDS wave RAM write while playing |
| FC043 | FDS disk / FDS wave | `fc.fds.bios.context_unsafe` | `error` | FDS BIOS call context unsafe |
| FC044 | FDS disk / FDS wave | `fc.fds.overlay.residency_mismatch` | `error` | FDS overlay residency mismatch |
| FC045 | VRC6 / VRC7 audio | `fc.vrc6.write_storm` | `warning` | VRC6 audio register write storm |
| FC046 | VRC6 / VRC7 audio | `fc.vrc6.phase_reset_risk` | `warning` | VRC6 phase reset risk |
| FC047 | VRC6 / VRC7 audio | `fc.vrc7.patch.sequence_broken` | `error` | VRC7 patch register sequence broken |
| FC048 | VRC6 / VRC7 audio | `fc.vrc7.init.incomplete` | `error` | VRC7 audio initialization incomplete |
| FC049 | VRC6 / VRC7 audio | `fc.exp_audio.mix.out_of_range` | `warning` | Expansion audio mix balance out of range |
| FC050 | stack / zero page / ABI | `fc.stack.overflow_risk` | `error` | Stack overflow or underflow risk |
| FC051 | stack / zero page / ABI | `fc.zp.alias_conflict` | `error` | Zero page alias conflict |
| FC052 | stack / zero page / ABI | `fc.abi.clobber` | `error` | Calling convention clobber |
| FC053 | stack / zero page / ABI | `fc.interrupt.temp_collision` | `error` | Interrupt temporary storage collision |
| FC054 | stack / zero page / ABI | `fc.farptr.metadata_mismatch` | `error` | Far pointer table metadata mismatch |
| FC055 | replay / snapshot / trace link | `fc.profiler.culprit.hotspot` | `info` | Profiler culprit ranking hotspot |
| FC056 | replay / snapshot / trace link | `fc.replay.divergence.irq_nmi` | `error` | Replay divergence at IRQ or NMI boundary |
| FC057 | replay / snapshot / trace link | `fc.trace.snapshot_missing` | `warning` | Snapshot or trace link missing |
| FC058 | replay / snapshot / trace link | `fc.repro_bundle.incomplete` | `warning` | Repro bundle incomplete |
| FC059 | replay / snapshot / trace link | `fc.compare.regression_unresolved` | `warning` | Before/after regression unresolved |


### FC001 PPU write outside VBlank or forced blank

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.ppu.write.unsafe` |
| Primary operation_kind | `ppu_write` |
| Fixture | `tests/fixtures/fc/fc001/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.ppu.write.unsafe` が出現する。
  - `pc` / bank / `operation_kind=ppu_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `ppu_op_id`, `nmi_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_state`, `addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.ppu.write.unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `ppu_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `ppu_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `move-to-nmi-vram-update-queue`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.ppu.write.unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.ppu.write.unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC001`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.ppu.write.unsafe` の正規化を追加する。
  2. `FC` metadata mapperで `ppu_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC001` ruleを追加する。
  4. repair hint generatorへ `move-to-nmi-vram-update-queue` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC002 Palette write glitch risk

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.palette.write.unsafe` |
| Primary operation_kind | `palette_write` |
| Fixture | `tests/fixtures/fc/fc002/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.palette.write.unsafe` が出現する。
  - `pc` / bank / `operation_kind=palette_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `palette_op_id`, `ppu_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_state`, `addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.palette.write.unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `palette_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `palette_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `queue-palette-write-during-vblank`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.palette.write.unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.palette.write.unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC002`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.palette.write.unsafe` の正規化を追加する。
  2. `FC` metadata mapperで `palette_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC002` ruleを追加する。
  4. repair hint generatorへ `queue-palette-write-during-vblank` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC003 Scroll write sequence broken

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.scroll.sequence.broken` |
| Primary operation_kind | `scroll_write` |
| Fixture | `tests/fixtures/fc/fc003/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.scroll.sequence.broken` が出現する。
  - `pc` / bank / `operation_kind=scroll_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `ppu_op_id`, `intrinsic_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `write_order`, `ppuscroll_latch`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.scroll.sequence.broken` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `scroll_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `scroll_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `write-ppuscroll-ppuaddr-in-stable-sequence`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.scroll.sequence.broken` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.scroll.sequence.broken` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC003`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.scroll.sequence.broken` の正規化を追加する。
  2. `FC` metadata mapperで `scroll_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC003` ruleを追加する。
  4. repair hint generatorへ `write-ppuscroll-ppuaddr-in-stable-sequence` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC004 PPUADDR latch desynchronization

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.ppuaddr.latch.desync` |
| Primary operation_kind | `ppuaddr_write` |
| Fixture | `tests/fixtures/fc/fc004/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.ppuaddr.latch.desync` が出現する。
  - `pc` / bank / `operation_kind=ppuaddr_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `ppu_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `latch_state`, `write_order`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.ppuaddr.latch.desync` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `ppuaddr_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `ppuaddr_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `reset-latch-with-ppustatus-read-before-address-sequence`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.ppuaddr.latch.desync` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.ppuaddr.latch.desync` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC004`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.ppuaddr.latch.desync` の正規化を追加する。
  2. `FC` metadata mapperで `ppuaddr_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC004` ruleを追加する。
  4. repair hint generatorへ `reset-latch-with-ppustatus-read-before-address-sequence` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC005 PPUCTRL NMI bit changed unsafely

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.ppuctrl.nmi_toggle.unsafe` |
| Primary operation_kind | `ppuctrl_write` |
| Fixture | `tests/fixtures/fc/fc005/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.ppuctrl.nmi_toggle.unsafe` が出現する。
  - `pc` / bank / `operation_kind=ppuctrl_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `nmi_handler_id`, `ppu_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppuctrl_before`, `ppuctrl_after`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.ppuctrl.nmi_toggle.unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `ppuctrl_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `ppuctrl_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `centralize-ppuctrl-shadow-and-apply-in-nmi`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.ppuctrl.nmi_toggle.unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.ppuctrl.nmi_toggle.unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC005`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.ppuctrl.nmi_toggle.unsafe` の正規化を追加する。
  2. `FC` metadata mapperで `ppuctrl_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC005` ruleを追加する。
  4. repair hint generatorへ `centralize-ppuctrl-shadow-and-apply-in-nmi` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC006 PPUMASK rendering toggle unsafe

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.ppumask.render_toggle.unsafe` |
| Primary operation_kind | `ppumask_write` |
| Fixture | `tests/fixtures/fc/fc006/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.ppumask.render_toggle.unsafe` が出現する。
  - `pc` / bank / `operation_kind=ppumask_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `ppu_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppumask_before`, `ppumask_after`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.ppumask.render_toggle.unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `ppumask_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `ppumask_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `toggle-rendering-at-frame-boundary`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.ppumask.render_toggle.unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.ppumask.render_toggle.unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC006`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.ppumask.render_toggle.unsafe` の正規化を追加する。
  2. `FC` metadata mapperで `ppumask_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC006` ruleを追加する。
  4. repair hint generatorへ `toggle-rendering-at-frame-boundary` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC007 VRAM update queue exceeds NMI budget

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `warning` |
| Primary event_type | `fc.vram_queue.over_budget` |
| Primary operation_kind | `vram_queue` |
| Fixture | `tests/fixtures/fc/fc007/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.vram_queue.over_budget` が出現する。
  - `pc` / bank / `operation_kind=vram_queue` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `ppu_op_id`, `nmi_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `queue_len`, `actual_cycles`, `budget_cycles`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.vram_queue.over_budget` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vram_queue` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vram_queue` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `split-vram-queue-across-frames`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.vram_queue.over_budget` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.vram_queue.over_budget` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC007`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.vram_queue.over_budget` の正規化を追加する。
  2. `FC` metadata mapperで `vram_queue` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC007` ruleを追加する。
  4. repair hint generatorへ `split-vram-queue-across-frames` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC008 Attribute table metadata mismatch

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `warning` |
| Primary event_type | `fc.attribute_table.mismatch` |
| Primary operation_kind | `attribute_write` |
| Fixture | `tests/fixtures/fc/fc008/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.attribute_table.mismatch` が出現する。
  - `pc` / bank / `operation_kind=attribute_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `ppu_op_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `nametable`, `attribute_addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.attribute_table.mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `attribute_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `attribute_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-attribute-table-generation-or-mirroring`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.attribute_table.mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.attribute_table.mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC008`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.attribute_table.mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `attribute_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC008` ruleを追加する。
  4. repair hint generatorへ `fix-attribute-table-generation-or-mirroring` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC009 CHR bank asset mismatch

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.chr_bank.asset_mismatch` |
| Primary operation_kind | `chr_asset` |
| Fixture | `tests/fixtures/fc/fc009/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.chr_bank.asset_mismatch` が出現する。
  - `pc` / bank / `operation_kind=chr_asset` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `expected_chr_bank`, `actual_chr_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.chr_bank.asset_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `chr_asset` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `chr_asset` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-chr-bank-binding-for-asset`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.chr_bank.asset_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.chr_bank.asset_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC009`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.chr_bank.asset_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `chr_asset` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC009` ruleを追加する。
  4. repair hint generatorへ `fix-chr-bank-binding-for-asset` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC010 Nametable mirroring mismatch

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `error` |
| Primary event_type | `fc.nametable.mirroring_mismatch` |
| Primary operation_kind | `mirroring` |
| Fixture | `tests/fixtures/fc/fc010/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.nametable.mirroring_mismatch` が出現する。
  - `pc` / bank / `operation_kind=mirroring` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `expected_mirroring`, `actual_mirroring`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.nametable.mirroring_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mirroring` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mirroring` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `configure-mirroring-before-nametable-write`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.nametable.mirroring_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.nametable.mirroring_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC010`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.nametable.mirroring_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `mirroring` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC010` ruleを追加する。
  4. repair hint generatorへ `configure-mirroring-before-nametable-write` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC011 Split scroll raster timing miss

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `warning` |
| Primary event_type | `fc.split_scroll.timing_miss` |
| Primary operation_kind | `split_scroll` |
| Fixture | `tests/fixtures/fc/fc011/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.split_scroll.timing_miss` が出現する。
  - `pc` / bank / `operation_kind=split_scroll` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `irq_handler_id`, `ppu_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `target_scanline`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.split_scroll.timing_miss` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `split_scroll` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `split_scroll` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `adjust-irq-timing-or-sprite0-wait`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.split_scroll.timing_miss` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.split_scroll.timing_miss` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC011`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.split_scroll.timing_miss` の正規化を追加する。
  2. `FC` metadata mapperで `split_scroll` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC011` ruleを追加する。
  4. repair hint generatorへ `adjust-irq-timing-or-sprite0-wait` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC012 Sprite zero hit timing unstable

| 項目 | 内容 |
|---|---|
| Category | PPU write / palette / scroll |
| Severity | `warning` |
| Primary event_type | `fc.sprite0_hit.unstable` |
| Primary operation_kind | `sprite0_hit` |
| Fixture | `tests/fixtures/fc/fc012/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.sprite0_hit.unstable` が出現する。
  - `pc` / bank / `operation_kind=sprite0_hit` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `ppu_op_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `hit_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.sprite0_hit.unstable` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `sprite0_hit` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `sprite0_hit` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `stabilize-sprite0-position-and-background-pixel`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.sprite0_hit.unstable` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.sprite0_hit.unstable` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC012`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.sprite0_hit.unstable` の正規化を追加する。
  2. `FC` metadata mapperで `sprite0_hit` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC012` ruleを追加する。
  4. repair hint generatorへ `stabilize-sprite0-position-and-background-pixel` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC013 OAM DMA outside safe window

| 項目 | 内容 |
|---|---|
| Category | OAM DMA / sprite overflow |
| Severity | `error` |
| Primary event_type | `fc.oamdma.unsafe_window` |
| Primary operation_kind | `oam_dma` |
| Fixture | `tests/fixtures/fc/fc013/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.oamdma.unsafe_window` が出現する。
  - `pc` / bank / `operation_kind=oam_dma` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `oam_dma_op_id`, `nmi_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dma_state`, `ppu_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.oamdma.unsafe_window` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `oam_dma` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `oam_dma` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `perform-oamdma-in-nmi-or-forced-blank`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.oamdma.unsafe_window` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.oamdma.unsafe_window` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC013`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.oamdma.unsafe_window` の正規化を追加する。
  2. `FC` metadata mapperで `oam_dma` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC013` ruleを追加する。
  4. repair hint generatorへ `perform-oamdma-in-nmi-or-forced-blank` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC014 OAM buffer overflow

| 項目 | 内容 |
|---|---|
| Category | OAM DMA / sprite overflow |
| Severity | `error` |
| Primary event_type | `fc.oam_buffer.overflow` |
| Primary operation_kind | `oam_buffer` |
| Fixture | `tests/fixtures/fc/fc014/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.oam_buffer.overflow` が出現する。
  - `pc` / bank / `operation_kind=oam_buffer` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `oam_dma_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `write_index`, `buffer_size`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.oam_buffer.overflow` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `oam_buffer` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `oam_buffer` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `clip-oam-writes-and-terminate-metasprite`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.oam_buffer.overflow` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.oam_buffer.overflow` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC014`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.oam_buffer.overflow` の正規化を追加する。
  2. `FC` metadata mapperで `oam_buffer` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC014` ruleを追加する。
  4. repair hint generatorへ `clip-oam-writes-and-terminate-metasprite` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC015 Sprite overflow on scanline

| 項目 | 内容 |
|---|---|
| Category | OAM DMA / sprite overflow |
| Severity | `warning` |
| Primary event_type | `fc.sprite.scanline.overflow` |
| Primary operation_kind | `sprite_eval` |
| Fixture | `tests/fixtures/fc/fc015/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.sprite.scanline.overflow` が出現する。
  - `pc` / bank / `operation_kind=sprite_eval` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `asset_id`, `oam_dma_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sprite_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.sprite.scanline.overflow` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `sprite_eval` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `sprite_eval` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `sort-or-drop-low-priority-sprites`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.sprite.scanline.overflow` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.sprite.scanline.overflow` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC015`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.sprite.scanline.overflow` の正規化を追加する。
  2. `FC` metadata mapperで `sprite_eval` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC015` ruleを追加する。
  4. repair hint generatorへ `sort-or-drop-low-priority-sprites` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC016 OAM DMA page crosses unexpected RAM page

| 項目 | 内容 |
|---|---|
| Category | OAM DMA / sprite overflow |
| Severity | `error` |
| Primary event_type | `fc.oamdma.page_mismatch` |
| Primary operation_kind | `oam_dma` |
| Fixture | `tests/fixtures/fc/fc016/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.oamdma.page_mismatch` が出現する。
  - `pc` / bank / `operation_kind=oam_dma` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `oam_dma_op_id`, `section`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dma_page`, `expected_page`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.oamdma.page_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `oam_dma` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `oam_dma` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `place-oam-buffer-on-fixed-256-byte-page`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.oamdma.page_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.oamdma.page_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC016`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.oamdma.page_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `oam_dma` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC016` ruleを追加する。
  4. repair hint generatorへ `place-oam-buffer-on-fixed-256-byte-page` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC017 Metasprite coordinate wrap risk

| 項目 | 内容 |
|---|---|
| Category | OAM DMA / sprite overflow |
| Severity | `warning` |
| Primary event_type | `fc.metasprite.coord_wrap` |
| Primary operation_kind | `metasprite_draw` |
| Fixture | `tests/fixtures/fc/fc017/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.metasprite.coord_wrap` が出現する。
  - `pc` / bank / `operation_kind=metasprite_draw` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `x`, `y`, `sprite_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.metasprite.coord_wrap` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `metasprite_draw` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `metasprite_draw` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `clip-coordinates-before-oam-fill`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.metasprite.coord_wrap` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.metasprite.coord_wrap` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC017`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.metasprite.coord_wrap` の正規化を追加する。
  2. `FC` metadata mapperで `metasprite_draw` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC017` ruleを追加する。
  4. repair hint generatorへ `clip-coordinates-before-oam-fill` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC018 Sprite tile CHR bank mismatch

| 項目 | 内容 |
|---|---|
| Category | OAM DMA / sprite overflow |
| Severity | `error` |
| Primary event_type | `fc.sprite.chr_bank_mismatch` |
| Primary operation_kind | `sprite_tile` |
| Fixture | `tests/fixtures/fc/fc018/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.sprite.chr_bank_mismatch` が出現する。
  - `pc` / bank / `operation_kind=sprite_tile` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `asset_id`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `tile_index`, `actual_chr_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.sprite.chr_bank_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `sprite_tile` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `sprite_tile` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `bind-sprite-asset-to-correct-chr-bank`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.sprite.chr_bank_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.sprite.chr_bank_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC018`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.sprite.chr_bank_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `sprite_tile` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC018` ruleを追加する。
  4. repair hint generatorへ `bind-sprite-asset-to-correct-chr-bank` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC019 OAM DMA / DMC conflict risk

| 項目 | 内容 |
|---|---|
| Category | OAM DMA / sprite overflow |
| Severity | `warning` |
| Primary event_type | `fc.oamdma.dmc_conflict` |
| Primary operation_kind | `oam_dma_dmc` |
| Fixture | `tests/fixtures/fc/fc019/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.oamdma.dmc_conflict` が出現する。
  - `pc` / bank / `operation_kind=oam_dma_dmc` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `oam_dma_op_id`, `dmc_read_risk_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dmc_state`, `dma_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.oamdma.dmc_conflict` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `oam_dma_dmc` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `oam_dma_dmc` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `schedule-oamdma-away-from-dmc-critical-read`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.oamdma.dmc_conflict` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.oamdma.dmc_conflict` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC019`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.oamdma.dmc_conflict` の正規化を追加する。
  2. `FC` metadata mapperで `oam_dma_dmc` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC019` ruleを追加する。
  4. repair hint generatorへ `schedule-oamdma-away-from-dmc-critical-read` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC020 NMI handler frame overrun

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `warning` |
| Primary event_type | `fc.nmi.frame_overrun` |
| Primary operation_kind | `nmi_handler` |
| Fixture | `tests/fixtures/fc/fc020/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.nmi.frame_overrun` が出現する。
  - `pc` / bank / `operation_kind=nmi_handler` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `nmi_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `actual_cycles`, `budget_cycles`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.nmi.frame_overrun` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `nmi_handler` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `nmi_handler` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `shorten-nmi-and-defer-work-to-main`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.nmi.frame_overrun` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.nmi.frame_overrun` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC020`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.nmi.frame_overrun` の正規化を追加する。
  2. `FC` metadata mapperで `nmi_handler` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC020` ruleを追加する。
  4. repair hint generatorへ `shorten-nmi-and-defer-work-to-main` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC021 NMI reentrancy or NMI during update

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `error` |
| Primary event_type | `fc.nmi.reentrant` |
| Primary operation_kind | `nmi_handler` |
| Fixture | `tests/fixtures/fc/fc021/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.nmi.reentrant` が出現する。
  - `pc` / bank / `operation_kind=nmi_handler` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `nmi_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `nmi_depth`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.nmi.reentrant` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `nmi_handler` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `nmi_handler` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `guard-nmi-critical-section-and-avoid-nested-state`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.nmi.reentrant` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.nmi.reentrant` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC021`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.nmi.reentrant` の正規化を追加する。
  2. `FC` metadata mapperで `nmi_handler` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC021` ruleを追加する。
  4. repair hint generatorへ `guard-nmi-critical-section-and-avoid-nested-state` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC022 IRQ acknowledge missing or late

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `error` |
| Primary event_type | `fc.irq.ack_late` |
| Primary operation_kind | `irq_handler` |
| Fixture | `tests/fixtures/fc/fc022/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.irq.ack_late` が出現する。
  - `pc` / bank / `operation_kind=irq_handler` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `irq_handler_id`, `mapper_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `irq_state`, `ack_cycle`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.irq.ack_late` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `irq_handler` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `irq_handler` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `acknowledge-irq-at-handler-entry`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.irq.ack_late` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.irq.ack_late` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC022`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.irq.ack_late` の正規化を追加する。
  2. `FC` metadata mapperで `irq_handler` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC022` ruleを追加する。
  4. repair hint generatorへ `acknowledge-irq-at-handler-entry` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC023 DMC DMA controller read conflict

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `warning` |
| Primary event_type | `fc.dmc.controller_conflict` |
| Primary operation_kind | `controller_read` |
| Fixture | `tests/fixtures/fc/fc023/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.dmc.controller_conflict` が出現する。
  - `pc` / bank / `operation_kind=controller_read` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dmc_read_risk_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dmc_state`, `controller_read_index`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.dmc.controller_conflict` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `controller_read` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `controller_read` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `use-dmc-safe-controller-read-routine`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.dmc.controller_conflict` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.dmc.controller_conflict` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC023`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.dmc.controller_conflict` の正規化を追加する。
  2. `FC` metadata mapperで `controller_read` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC023` ruleを追加する。
  4. repair hint generatorへ `use-dmc-safe-controller-read-routine` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC024 Controller double clock or misread

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `error` |
| Primary event_type | `fc.controller.double_clock` |
| Primary operation_kind | `controller_read` |
| Fixture | `tests/fixtures/fc/fc024/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.controller.double_clock` が出現する。
  - `pc` / bank / `operation_kind=controller_read` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `read_sequence`, `latched_value`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.controller.double_clock` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `controller_read` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `controller_read` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-controller-strobe-and-8-read-loop`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.controller.double_clock` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.controller.double_clock` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC024`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.controller.double_clock` の正規化を追加する。
  2. `FC` metadata mapperで `controller_read` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC024` ruleを追加する。
  4. repair hint generatorへ `fix-controller-strobe-and-8-read-loop` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC025 NMI/main thread queue race

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `error` |
| Primary event_type | `fc.nmi_main.queue_race` |
| Primary operation_kind | `nmi_queue` |
| Fixture | `tests/fixtures/fc/fc025/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.nmi_main.queue_race` が出現する。
  - `pc` / bank / `operation_kind=nmi_queue` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `nmi_handler_id`, `ppu_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `queue_head`, `queue_tail`, `race_site`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.nmi_main.queue_race` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `nmi_queue` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `nmi_queue` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `use-atomic-queue-swap-or-disable-nmi-briefly`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.nmi_main.queue_race` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.nmi_main.queue_race` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC025`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.nmi_main.queue_race` の正規化を追加する。
  2. `FC` metadata mapperで `nmi_queue` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC025` ruleを追加する。
  4. repair hint generatorへ `use-atomic-queue-swap-or-disable-nmi-briefly` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC026 IRQ vector bank mismatch

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `error` |
| Primary event_type | `fc.irq.vector_bank_mismatch` |
| Primary operation_kind | `irq_vector` |
| Fixture | `tests/fixtures/fc/fc026/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.irq.vector_bank_mismatch` が出現する。
  - `pc` / bank / `operation_kind=irq_vector` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `irq_handler_id`, `prg_bank`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `expected_bank`, `actual_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.irq.vector_bank_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `irq_vector` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `irq_vector` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `pin-irq-handler-in-fixed-bank`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.irq.vector_bank_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.irq.vector_bank_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC026`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.irq.vector_bank_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `irq_vector` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC026` ruleを追加する。
  4. repair hint generatorへ `pin-irq-handler-in-fixed-bank` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC027 VBlank flag stale read

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `warning` |
| Primary event_type | `fc.vblank_flag.stale` |
| Primary operation_kind | `vblank_flag` |
| Fixture | `tests/fixtures/fc/fc027/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.vblank_flag.stale` が出現する。
  - `pc` / bank / `operation_kind=vblank_flag` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `nmi_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `flag_value`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.vblank_flag.stale` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vblank_flag` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vblank_flag` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `clear-and-set-vblank-flag-with-monotonic-frame-counter`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.vblank_flag.stale` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.vblank_flag.stale` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC027`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.vblank_flag.stale` の正規化を追加する。
  2. `FC` metadata mapperで `vblank_flag` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC027` ruleを追加する。
  4. repair hint generatorへ `clear-and-set-vblank-flag-with-monotonic-frame-counter` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC028 APU frame counter timing mismatch

| 項目 | 内容 |
|---|---|
| Category | NMI / IRQ / DMC / controller |
| Severity | `warning` |
| Primary event_type | `fc.apu.frame_counter_mismatch` |
| Primary operation_kind | `apu_frame_counter` |
| Fixture | `tests/fixtures/fc/fc028/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.apu.frame_counter_mismatch` が出現する。
  - `pc` / bank / `operation_kind=apu_frame_counter` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `apu_state`, `expected_step`, `actual_step`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.apu.frame_counter_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `apu_frame_counter` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `apu_frame_counter` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `centralize-apu-frame-counter-init`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.apu.frame_counter_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.apu.frame_counter_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC028`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.apu.frame_counter_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `apu_frame_counter` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC028` ruleを追加する。
  4. repair hint generatorへ `centralize-apu-frame-counter-init` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC029 Mapper write goes to wrong bank/register

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.mapper.write.wrong_bank` |
| Primary operation_kind | `mapper_write` |
| Fixture | `tests/fixtures/fc/fc029/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.mapper.write.wrong_bank` が出現する。
  - `pc` / bank / `operation_kind=mapper_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `prg_bank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `mapper_state`, `addr`, `value`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.mapper.write.wrong_bank` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mapper_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mapper_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-mapper-register-address-or-bank-shadow`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.mapper.write.wrong_bank` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.mapper.write.wrong_bank` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC029`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.mapper.write.wrong_bank` の正規化を追加する。
  2. `FC` metadata mapperで `mapper_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC029` ruleを追加する。
  4. repair hint generatorへ `fix-mapper-register-address-or-bank-shadow` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC030 MMC1 serial write interrupted

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.mmc1.serial.interrupted` |
| Primary operation_kind | `mmc1_write` |
| Fixture | `tests/fixtures/fc/fc030/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.mmc1.serial.interrupted` が出現する。
  - `pc` / bank / `operation_kind=mmc1_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `serial_count`, `interrupt_seen`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.mmc1.serial.interrupted` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mmc1_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mmc1_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `protect-mmc1-5-write-sequence-from-interrupts`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.mmc1.serial.interrupted` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.mmc1.serial.interrupted` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC030`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.mmc1.serial.interrupted` の正規化を追加する。
  2. `FC` metadata mapperで `mmc1_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC030` ruleを追加する。
  4. repair hint generatorへ `protect-mmc1-5-write-sequence-from-interrupts` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC031 MMC3 scanline IRQ timing mismatch

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.mmc3.irq.timing_mismatch` |
| Primary operation_kind | `mmc3_irq` |
| Fixture | `tests/fixtures/fc/fc031/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.mmc3.irq.timing_mismatch` が出現する。
  - `pc` / bank / `operation_kind=mmc3_irq` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `irq_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `irq_counter`, `a12_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.mmc3.irq.timing_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mmc3_irq` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mmc3_irq` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `adjust-mmc3-irq-setup-and-ack-order`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.mmc3.irq.timing_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.mmc3.irq.timing_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC031`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.mmc3.irq.timing_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `mmc3_irq` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC031` ruleを追加する。
  4. repair hint generatorへ `adjust-mmc3-irq-setup-and-ack-order` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC032 MMC3 A12 filter false trigger

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `warning` |
| Primary event_type | `fc.mmc3.a12.false_trigger` |
| Primary operation_kind | `mmc3_a12` |
| Fixture | `tests/fixtures/fc/fc032/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.mmc3.a12.false_trigger` が出現する。
  - `pc` / bank / `operation_kind=mmc3_a12` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `a12_edge_count`, `filter_window`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.mmc3.a12.false_trigger` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mmc3_a12` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mmc3_a12` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `stabilize-chr-layout-or-delay-counter-reload`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.mmc3.a12.false_trigger` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.mmc3.a12.false_trigger` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC032`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.mmc3.a12.false_trigger` の正規化を追加する。
  2. `FC` metadata mapperで `mmc3_a12` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC032` ruleを追加する。
  4. repair hint generatorへ `stabilize-chr-layout-or-delay-counter-reload` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC033 MMC5 ExRAM mode mismatch

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.mmc5.exram.mode_mismatch` |
| Primary operation_kind | `mmc5_exram` |
| Fixture | `tests/fixtures/fc/fc033/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.mmc5.exram.mode_mismatch` が出現する。
  - `pc` / bank / `operation_kind=mmc5_exram` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `expected_mode`, `actual_mode`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.mmc5.exram.mode_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mmc5_exram` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mmc5_exram` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `set-exram-mode-before-nametable-or-attribute-use`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.mmc5.exram.mode_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.mmc5.exram.mode_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC033`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.mmc5.exram.mode_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `mmc5_exram` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC033` ruleを追加する。
  4. repair hint generatorへ `set-exram-mode-before-nametable-or-attribute-use` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC034 Banked call without trampoline

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.banked_call.no_trampoline` |
| Primary operation_kind | `banked_call` |
| Fixture | `tests/fixtures/fc/fc034/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.banked_call.no_trampoline` が出現する。
  - `pc` / bank / `operation_kind=banked_call` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `prg_bank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `target_pc`, `actual_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.banked_call.no_trampoline` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `banked_call` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `banked_call` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `replace-direct-call-with-banked-trampoline`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.banked_call.no_trampoline` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.banked_call.no_trampoline` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC034`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.banked_call.no_trampoline` の正規化を追加する。
  2. `FC` metadata mapperで `banked_call` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC034` ruleを追加する。
  4. repair hint generatorへ `replace-direct-call-with-banked-trampoline` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC035 PRG bank restore missing

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.prg_bank.restore_missing` |
| Primary operation_kind | `bank_switch` |
| Fixture | `tests/fixtures/fc/fc035/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.prg_bank.restore_missing` が出現する。
  - `pc` / bank / `operation_kind=bank_switch` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `prg_bank_before`, `prg_bank_after`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.prg_bank.restore_missing` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `bank_switch` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `bank_switch` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `restore-prg-bank-on-all-exit-paths`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.prg_bank.restore_missing` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.prg_bank.restore_missing` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC035`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.prg_bank.restore_missing` の正規化を追加する。
  2. `FC` metadata mapperで `bank_switch` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC035` ruleを追加する。
  4. repair hint generatorへ `restore-prg-bank-on-all-exit-paths` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC036 CHR bank swapped during rendering

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.chr_bank.swap_during_render` |
| Primary operation_kind | `chr_bank_switch` |
| Fixture | `tests/fixtures/fc/fc036/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.chr_bank.swap_during_render` が出現する。
  - `pc` / bank / `operation_kind=chr_bank_switch` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_op_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_state`, `chr_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.chr_bank.swap_during_render` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `chr_bank_switch` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `chr_bank_switch` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `perform-chr-swap-in-vblank-or-planned-raster-window`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.chr_bank.swap_during_render` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.chr_bank.swap_during_render` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC036`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.chr_bank.swap_during_render` の正規化を追加する。
  2. `FC` metadata mapperで `chr_bank_switch` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC036` ruleを追加する。
  4. repair hint generatorへ `perform-chr-swap-in-vblank-or-planned-raster-window` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC037 Mapper register shadow desync

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `warning` |
| Primary event_type | `fc.mapper.shadow_desync` |
| Primary operation_kind | `mapper_shadow` |
| Fixture | `tests/fixtures/fc/fc037/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.mapper.shadow_desync` が出現する。
  - `pc` / bank / `operation_kind=mapper_shadow` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `shadow_value`, `actual_value`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.mapper.shadow_desync` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mapper_shadow` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mapper_shadow` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `update-shadow-on-every-mapper-write`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.mapper.shadow_desync` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.mapper.shadow_desync` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC037`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.mapper.shadow_desync` の正規化を追加する。
  2. `FC` metadata mapperで `mapper_shadow` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC037` ruleを追加する。
  4. repair hint generatorへ `update-shadow-on-every-mapper-write` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC038 Unsupported mapper feature used

| 項目 | 内容 |
|---|---|
| Category | mapper / MMC / bank switch |
| Severity | `error` |
| Primary event_type | `fc.mapper.unsupported_feature` |
| Primary operation_kind | `mapper_feature` |
| Fixture | `tests/fixtures/fc/fc038/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.mapper.unsupported_feature` が出現する。
  - `pc` / bank / `operation_kind=mapper_feature` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_id`, `mapper_op_id`, `build_warning_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `feature_name`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.mapper.unsupported_feature` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `mapper_feature` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `mapper_feature` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `add-emulator-support-or-avoid-feature`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.mapper.unsupported_feature` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.mapper.unsupported_feature` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC038`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.mapper.unsupported_feature` の正規化を追加する。
  2. `FC` metadata mapperで `mapper_feature` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC038` ruleを追加する。
  4. repair hint generatorへ `add-emulator-support-or-avoid-feature` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC039 FDS disk side state mismatch

| 項目 | 内容 |
|---|---|
| Category | FDS disk / FDS wave |
| Severity | `error` |
| Primary event_type | `fc.fds.disk_side.mismatch` |
| Primary operation_kind | `fds_disk` |
| Fixture | `tests/fixtures/fc/fc039/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.fds.disk_side.mismatch` が出現する。
  - `pc` / bank / `operation_kind=fds_disk` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `fds_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `expected_side`, `actual_side`, `fds_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.fds.disk_side.mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `fds_disk` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `fds_disk` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `synchronize-disk-side-before-file-load`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.fds.disk_side.mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.fds.disk_side.mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC039`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.fds.disk_side.mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `fds_disk` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC039` ruleを追加する。
  4. repair hint generatorへ `synchronize-disk-side-before-file-load` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC040 FDS disk read timing risk

| 項目 | 内容 |
|---|---|
| Category | FDS disk / FDS wave |
| Severity | `warning` |
| Primary event_type | `fc.fds.read.timing_risk` |
| Primary operation_kind | `fds_read` |
| Fixture | `tests/fixtures/fc/fc040/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.fds.read.timing_risk` が出現する。
  - `pc` / bank / `operation_kind=fds_read` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `fds_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `fds_state`, `read_latency`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.fds.read.timing_risk` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `fds_read` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `fds_read` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `insert-state-machine-wait-for-disk-ready`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.fds.read.timing_risk` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.fds.read.timing_risk` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC040`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.fds.read.timing_risk` の正規化を追加する。
  2. `FC` metadata mapperで `fds_read` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC040` ruleを追加する。
  4. repair hint generatorへ `insert-state-machine-wait-for-disk-ready` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC041 FDS IRQ transfer unacknowledged

| 項目 | 内容 |
|---|---|
| Category | FDS disk / FDS wave |
| Severity | `error` |
| Primary event_type | `fc.fds.irq.unacked` |
| Primary operation_kind | `fds_irq` |
| Fixture | `tests/fixtures/fc/fc041/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.fds.irq.unacked` が出現する。
  - `pc` / bank / `operation_kind=fds_irq` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `fds_op_id`, `irq_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `irq_state`, `fds_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.fds.irq.unacked` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `fds_irq` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `fds_irq` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `ack-fds-irq-and-clear-transfer-state`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.fds.irq.unacked` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.fds.irq.unacked` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC041`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.fds.irq.unacked` の正規化を追加する。
  2. `FC` metadata mapperで `fds_irq` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC041` ruleを追加する。
  4. repair hint generatorへ `ack-fds-irq-and-clear-transfer-state` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC042 FDS wave RAM write while playing

| 項目 | 内容 |
|---|---|
| Category | FDS disk / FDS wave |
| Severity | `error` |
| Primary event_type | `fc.fds.wave.write_while_playing` |
| Primary operation_kind | `fds_wave_write` |
| Fixture | `tests/fixtures/fc/fc042/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.fds.wave.write_while_playing` が出現する。
  - `pc` / bank / `operation_kind=fds_wave_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `fds_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `fds_state`, `wave_addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.fds.wave.write_while_playing` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `fds_wave_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `fds_wave_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `disable-fds-wave-before-wave-ram-update`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.fds.wave.write_while_playing` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.fds.wave.write_while_playing` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC042`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.fds.wave.write_while_playing` の正規化を追加する。
  2. `FC` metadata mapperで `fds_wave_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC042` ruleを追加する。
  4. repair hint generatorへ `disable-fds-wave-before-wave-ram-update` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC043 FDS BIOS call context unsafe

| 項目 | 内容 |
|---|---|
| Category | FDS disk / FDS wave |
| Severity | `error` |
| Primary event_type | `fc.fds.bios.context_unsafe` |
| Primary operation_kind | `fds_bios_call` |
| Fixture | `tests/fixtures/fc/fc043/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.fds.bios.context_unsafe` が出現する。
  - `pc` / bank / `operation_kind=fds_bios_call` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `fds_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sp`, `prg_bank`, `irq_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.fds.bios.context_unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `fds_bios_call` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `fds_bios_call` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `wrap-bios-call-with-known-safe-context`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.fds.bios.context_unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.fds.bios.context_unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC043`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.fds.bios.context_unsafe` の正規化を追加する。
  2. `FC` metadata mapperで `fds_bios_call` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC043` ruleを追加する。
  4. repair hint generatorへ `wrap-bios-call-with-known-safe-context` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC044 FDS overlay residency mismatch

| 項目 | 内容 |
|---|---|
| Category | FDS disk / FDS wave |
| Severity | `error` |
| Primary event_type | `fc.fds.overlay.residency_mismatch` |
| Primary operation_kind | `fds_overlay` |
| Fixture | `tests/fixtures/fc/fc044/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.fds.overlay.residency_mismatch` が出現する。
  - `pc` / bank / `operation_kind=fds_overlay` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `fds_op_id`, `section`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `overlay_id`, `resident`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.fds.overlay.residency_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `fds_overlay` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `fds_overlay` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `load-required-overlay-before-call`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.fds.overlay.residency_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.fds.overlay.residency_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC044`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.fds.overlay.residency_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `fds_overlay` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC044` ruleを追加する。
  4. repair hint generatorへ `load-required-overlay-before-call` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC045 VRC6 audio register write storm

| 項目 | 内容 |
|---|---|
| Category | VRC6 / VRC7 audio |
| Severity | `warning` |
| Primary event_type | `fc.vrc6.write_storm` |
| Primary operation_kind | `vrc6_audio` |
| Fixture | `tests/fixtures/fc/fc045/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.vrc6.write_storm` が出現する。
  - `pc` / bank / `operation_kind=vrc6_audio` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `vrc6_audio_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `vrc_state`, `write_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.vrc6.write_storm` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vrc6_audio` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vrc6_audio` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `rate-limit-vrc6-register-updates`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.vrc6.write_storm` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.vrc6.write_storm` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC045`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.vrc6.write_storm` の正規化を追加する。
  2. `FC` metadata mapperで `vrc6_audio` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC045` ruleを追加する。
  4. repair hint generatorへ `rate-limit-vrc6-register-updates` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC046 VRC6 phase reset risk

| 項目 | 内容 |
|---|---|
| Category | VRC6 / VRC7 audio |
| Severity | `warning` |
| Primary event_type | `fc.vrc6.phase_reset_risk` |
| Primary operation_kind | `vrc6_audio` |
| Fixture | `tests/fixtures/fc/fc046/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.vrc6.phase_reset_risk` が出現する。
  - `pc` / bank / `operation_kind=vrc6_audio` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `vrc6_audio_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `channel`, `period_before`, `period_after`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.vrc6.phase_reset_risk` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vrc6_audio` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vrc6_audio` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `avoid-unnecessary-period-rewrites`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.vrc6.phase_reset_risk` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.vrc6.phase_reset_risk` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC046`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.vrc6.phase_reset_risk` の正規化を追加する。
  2. `FC` metadata mapperで `vrc6_audio` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC046` ruleを追加する。
  4. repair hint generatorへ `avoid-unnecessary-period-rewrites` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC047 VRC7 patch register sequence broken

| 項目 | 内容 |
|---|---|
| Category | VRC6 / VRC7 audio |
| Severity | `error` |
| Primary event_type | `fc.vrc7.patch.sequence_broken` |
| Primary operation_kind | `vrc7_audio` |
| Fixture | `tests/fixtures/fc/fc047/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.vrc7.patch.sequence_broken` が出現する。
  - `pc` / bank / `operation_kind=vrc7_audio` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `vrc7_audio_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `register_sequence`, `vrc_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.vrc7.patch.sequence_broken` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vrc7_audio` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vrc7_audio` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `emit-vrc7-patch-registers-in-required-order`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.vrc7.patch.sequence_broken` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.vrc7.patch.sequence_broken` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC047`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.vrc7.patch.sequence_broken` の正規化を追加する。
  2. `FC` metadata mapperで `vrc7_audio` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC047` ruleを追加する。
  4. repair hint generatorへ `emit-vrc7-patch-registers-in-required-order` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC048 VRC7 audio initialization incomplete

| 項目 | 内容 |
|---|---|
| Category | VRC6 / VRC7 audio |
| Severity | `error` |
| Primary event_type | `fc.vrc7.init.incomplete` |
| Primary operation_kind | `vrc7_audio` |
| Fixture | `tests/fixtures/fc/fc048/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.vrc7.init.incomplete` が出現する。
  - `pc` / bank / `operation_kind=vrc7_audio` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `vrc7_audio_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `missing_registers`, `vrc_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.vrc7.init.incomplete` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vrc7_audio` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vrc7_audio` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `complete-vrc7-init-before-note-on`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.vrc7.init.incomplete` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.vrc7.init.incomplete` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC048`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.vrc7.init.incomplete` の正規化を追加する。
  2. `FC` metadata mapperで `vrc7_audio` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC048` ruleを追加する。
  4. repair hint generatorへ `complete-vrc7-init-before-note-on` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC049 Expansion audio mix balance out of range

| 項目 | 内容 |
|---|---|
| Category | VRC6 / VRC7 audio |
| Severity | `warning` |
| Primary event_type | `fc.exp_audio.mix.out_of_range` |
| Primary operation_kind | `expansion_audio` |
| Fixture | `tests/fixtures/fc/fc049/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.exp_audio.mix.out_of_range` が出現する。
  - `pc` / bank / `operation_kind=expansion_audio` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `vrc6_audio_op_id`, `vrc7_audio_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `mix_level`, `apu_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.exp_audio.mix.out_of_range` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `expansion_audio` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `expansion_audio` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `clamp-expansion-audio-mix-parameters`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.exp_audio.mix.out_of_range` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.exp_audio.mix.out_of_range` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC049`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.exp_audio.mix.out_of_range` の正規化を追加する。
  2. `FC` metadata mapperで `expansion_audio` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC049` ruleを追加する。
  4. repair hint generatorへ `clamp-expansion-audio-mix-parameters` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC050 Stack overflow or underflow risk

| 項目 | 内容 |
|---|---|
| Category | stack / zero page / ABI |
| Severity | `error` |
| Primary event_type | `fc.stack.overflow_risk` |
| Primary operation_kind | `stack_usage` |
| Fixture | `tests/fixtures/fc/fc050/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.stack.overflow_risk` が出現する。
  - `pc` / bank / `operation_kind=stack_usage` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `stack_usage`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sp_min`, `sp_max`, `stack_reserve`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.stack.overflow_risk` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `stack_usage` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `stack_usage` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `reduce-stack-use-or-move-buffer-to-bss`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.stack.overflow_risk` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.stack.overflow_risk` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC050`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.stack.overflow_risk` の正規化を追加する。
  2. `FC` metadata mapperで `stack_usage` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC050` ruleを追加する。
  4. repair hint generatorへ `reduce-stack-use-or-move-buffer-to-bss` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC051 Zero page alias conflict

| 項目 | 内容 |
|---|---|
| Category | stack / zero page / ABI |
| Severity | `error` |
| Primary event_type | `fc.zp.alias_conflict` |
| Primary operation_kind | `zero_page` |
| Fixture | `tests/fixtures/fc/fc051/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.zp.alias_conflict` が出現する。
  - `pc` / bank / `operation_kind=zero_page` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `zero_page_usage`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `zp_addr`, `owner_a`, `owner_b`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.zp.alias_conflict` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `zero_page` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `zero_page` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `reassign-zero-page-slot`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.zp.alias_conflict` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.zp.alias_conflict` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC051`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.zp.alias_conflict` の正規化を追加する。
  2. `FC` metadata mapperで `zero_page` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC051` ruleを追加する。
  4. repair hint generatorへ `reassign-zero-page-slot` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC052 Calling convention clobber

| 項目 | 内容 |
|---|---|
| Category | stack / zero page / ABI |
| Severity | `error` |
| Primary event_type | `fc.abi.clobber` |
| Primary operation_kind | `abi_check` |
| Fixture | `tests/fixtures/fc/fc052/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.abi.clobber` が出現する。
  - `pc` / bank / `operation_kind=abi_check` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `register`, `expected`, `actual`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.abi.clobber` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `abi_check` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `abi_check` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `preserve-register-or-fix-abi-metadata`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.abi.clobber` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.abi.clobber` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC052`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.abi.clobber` の正規化を追加する。
  2. `FC` metadata mapperで `abi_check` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC052` ruleを追加する。
  4. repair hint generatorへ `preserve-register-or-fix-abi-metadata` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC053 Interrupt temporary storage collision

| 項目 | 内容 |
|---|---|
| Category | stack / zero page / ABI |
| Severity | `error` |
| Primary event_type | `fc.interrupt.temp_collision` |
| Primary operation_kind | `interrupt_temp` |
| Fixture | `tests/fixtures/fc/fc053/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.interrupt.temp_collision` が出現する。
  - `pc` / bank / `operation_kind=interrupt_temp` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `zero_page_usage`, `nmi_handler_id`, `irq_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `temp_addr`, `owner_a`, `owner_b`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.interrupt.temp_collision` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `interrupt_temp` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `interrupt_temp` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `separate-main-nmi-irq-temp-storage`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.interrupt.temp_collision` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.interrupt.temp_collision` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC053`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.interrupt.temp_collision` の正規化を追加する。
  2. `FC` metadata mapperで `interrupt_temp` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC053` ruleを追加する。
  4. repair hint generatorへ `separate-main-nmi-irq-temp-storage` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC054 Far pointer table metadata mismatch

| 項目 | 内容 |
|---|---|
| Category | stack / zero page / ABI |
| Severity | `error` |
| Primary event_type | `fc.farptr.metadata_mismatch` |
| Primary operation_kind | `far_pointer` |
| Fixture | `tests/fixtures/fc/fc054/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.farptr.metadata_mismatch` が出現する。
  - `pc` / bank / `operation_kind=far_pointer` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `prg_bank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `table_index`, `expected_bank`, `actual_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.farptr.metadata_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `far_pointer` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `far_pointer` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `regenerate-far-pointer-table`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.farptr.metadata_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.farptr.metadata_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC054`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.farptr.metadata_mismatch` の正規化を追加する。
  2. `FC` metadata mapperで `far_pointer` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC054` ruleを追加する。
  4. repair hint generatorへ `regenerate-far-pointer-table` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC055 Profiler culprit ranking hotspot

| 項目 | 内容 |
|---|---|
| Category | replay / snapshot / trace link |
| Severity | `info` |
| Primary event_type | `fc.profiler.culprit.hotspot` |
| Primary operation_kind | `profiling` |
| Fixture | `tests/fixtures/fc/fc055/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.profiler.culprit.hotspot` が出現する。
  - `pc` / bank / `operation_kind=profiling` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sample_count`, `exclusive_cycles`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.profiler.culprit.hotspot` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `profiling` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `profiling` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `surface-hotspot-before-code-change`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.profiler.culprit.hotspot` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.profiler.culprit.hotspot` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC055`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.profiler.culprit.hotspot` の正規化を追加する。
  2. `FC` metadata mapperで `profiling` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC055` ruleを追加する。
  4. repair hint generatorへ `surface-hotspot-before-code-change` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC056 Replay divergence at IRQ or NMI boundary

| 項目 | 内容 |
|---|---|
| Category | replay / snapshot / trace link |
| Severity | `error` |
| Primary event_type | `fc.replay.divergence.irq_nmi` |
| Primary operation_kind | `replay` |
| Fixture | `tests/fixtures/fc/fc056/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.replay.divergence.irq_nmi` が出現する。
  - `pc` / bank / `operation_kind=replay` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `nmi_handler_id`, `irq_handler_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `replay_step`, `expected_hash`, `actual_hash`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.replay.divergence.irq_nmi` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `replay` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `replay` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `attach-boundary-trace-and-stabilize-nondeterministic-input`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.replay.divergence.irq_nmi` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.replay.divergence.irq_nmi` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC056`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.replay.divergence.irq_nmi` の正規化を追加する。
  2. `FC` metadata mapperで `replay` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC056` ruleを追加する。
  4. repair hint generatorへ `attach-boundary-trace-and-stabilize-nondeterministic-input` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC057 Snapshot or trace link missing

| 項目 | 内容 |
|---|---|
| Category | replay / snapshot / trace link |
| Severity | `warning` |
| Primary event_type | `fc.trace.snapshot_missing` |
| Primary operation_kind | `trace_link` |
| Fixture | `tests/fixtures/fc/fc057/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.trace.snapshot_missing` が出現する。
  - `pc` / bank / `operation_kind=trace_link` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `snapshot_ref`, `trace_window_ref`, `count`, `first_seen`, `last_seen`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.trace.snapshot_missing` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `trace_link` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `trace_link` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `emit-snapshot-and-trace-window-for-event`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.trace.snapshot_missing` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.trace.snapshot_missing` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC057`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.trace.snapshot_missing` の正規化を追加する。
  2. `FC` metadata mapperで `trace_link` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC057` ruleを追加する。
  4. repair hint generatorへ `emit-snapshot-and-trace-window-for-event` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC058 Repro bundle incomplete

| 項目 | 内容 |
|---|---|
| Category | replay / snapshot / trace link |
| Severity | `warning` |
| Primary event_type | `fc.repro_bundle.incomplete` |
| Primary operation_kind | `repro_bundle` |
| Fixture | `tests/fixtures/fc/fc058/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.repro_bundle.incomplete` が出現する。
  - `pc` / bank / `operation_kind=repro_bundle` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `build_warning_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `missing_file`, `manifest_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.repro_bundle.incomplete` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `repro_bundle` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `repro_bundle` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `include-required-events-metadata-and-command-files`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.repro_bundle.incomplete` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.repro_bundle.incomplete` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC058`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.repro_bundle.incomplete` の正規化を追加する。
  2. `FC` metadata mapperで `repro_bundle` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC058` ruleを追加する。
  4. repair hint generatorへ `include-required-events-metadata-and-command-files` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### FC059 Before/after regression unresolved

| 項目 | 内容 |
|---|---|
| Category | replay / snapshot / trace link |
| Severity | `warning` |
| Primary event_type | `fc.compare.regression_unresolved` |
| Primary operation_kind | `before_after_compare` |
| Fixture | `tests/fixtures/fc/fc059/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `fc.compare.regression_unresolved` が出現する。
  - `pc` / bank / `operation_kind=before_after_compare` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `mapper_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `before_count`, `after_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `fc.compare.regression_unresolved` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `before_after_compare` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `before_after_compare` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|nmi_queue|mapper_setup|asset_metadata|audio_update|fds_overlay` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `block-release-until-regression-is-explained`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `fc.compare.regression_unresolved` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `fc.compare.regression_unresolved` を1件以上含める。
  - `expected_ai_diagnostics.json`: `FC059`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `FC` event parserへ `fc.compare.regression_unresolved` の正規化を追加する。
  2. `FC` metadata mapperで `before_after_compare` とmetadata idの対応付けを実装する。
  3. `diagnostics_fc.rs` に `FC059` ruleを追加する。
  4. repair hint generatorへ `block-release-until-regression-is-explained` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

## 11. 完了条件

- Rust workspace が作成され、`cargo test --workspace` が通る。
- `fc` analyze CLIがmetadata JSONとdiagnostic_events.jsonlを読み込める。
- 本書の 59件すべてに rule、fixture、expected output が存在する。
- `diagnostic_summary.json`、`ai_diagnostics.json`、`report.html`、`repair_prompt.md`、`repro_bundle.zip` が生成される。
- `sarakura validate` が生成物をschema検証できる。
- `sarakura compare` が before/after の resolved / improved / unchanged / regressed / new を分類できる。
- すべての診断で `evidence_chain`、`target_candidates`、`repair_target`、`safe_patch_hint`、`confidence`、`retest_condition` が欠落しない。
- JSONL入力はstreaming処理され、巨大JSON配列を要求しない。

## 12. Codexへの実装順序

1. workspaceとcrate雛形を作成する。
2. 共通modelとschemaを `sarakura-core` / `sarakura-schema` に実装する。
3. target別metadata/events parserを実装する。
4. 代表診断3件だけを先に通す。
5. HTML reportとrepair_promptの最小出力を作る。
6. repro bundleを作る。
7. 残り診断をカテゴリ単位で追加する。
8. before/after compareを追加する。
9. 全fixtureとsnapshot testをそろえる。
10. README/CLI.md/REPORT_FORMAT.mdを整備する。

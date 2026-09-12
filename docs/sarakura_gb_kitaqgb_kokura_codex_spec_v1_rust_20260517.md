# SARAKURA GB / KITAQGB + KOKURA Codex Spec v1 Rust

作成日: 2026-05-17  
対象: KITAQGB / KOKURA / Game Boy / Game Boy Color  
文書種別: Codex実装仕様書 / SARAKURA Rust v1  
診断件数: 48件  

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

## 3. 入力仕様: KITAQGB / KOKURA

### 3.1 KITAQGB build metadata

ファイル名の既定値は `kitaqgb_build_metadata.json`。

必須・推奨フィールド：

- `source_file`
- `source_line`
- `function_id`
- `function_name`
- `rom_bank`
- `pc_range`
- `symbol_name`
- `intrinsic_id`
- `intrinsic_kind`
- `operation_kind`
- `requires_vblank`
- `vram_op_id`
- `oam_op_id`
- `dma_op_id`
- `apu_update_id`
- `bank_switch_id`
- `farcall_id`
- `asset_id`
- `tilemap_id`
- `sprite_id`
- `bgm_id`
- `section`
- `wram_bank`
- `hram_usage`
- `queue_id`
- `abi.sp_expected`
- `abi.register_preserve`
- `build_warning_id`

最小例：

```json
{
  "schema_version": "kitaqgb.build_metadata.v1",
  "target": "gbc",
  "functions": [
    {
      "function_id": "fn_draw_board",
      "function_name": "draw_board",
      "source_file": "src/main.c",
      "source_line": 120,
      "rom_bank": 3,
      "pc_range": { "start": 49152, "end": 49320 },
      "symbol_name": "_draw_board"
    }
  ],
  "operations": []
}
```

### 3.2 KOKURA diagnostic events

ファイル名の既定値は `kokura_diagnostic_events.jsonl`。1行1イベント。巨大JSON配列は禁止する。

必須・推奨フィールド：

- `event_id`
- `event_type`
- `severity`
- `frame`
- `scanline`
- `dot`
- `cycle`
- `pc`
- `rom_bank`
- `addr`
- `access_kind`
- `ppu_mode`
- `lcdc_state`
- `stat_state`
- `dma_state`
- `apu_state`
- `function_hint`
- `symbol_hint`
- `count`
- `first_seen`
- `last_seen`
- `snapshot_ref`
- `trace_window_ref`

最小例：

```jsonl
{"event_id":"ev1","event_type":"gb.vram.write.blocked","severity":"error","frame":12,"scanline":80,"dot":12,"pc":49180,"rom_bank":3,"addr":38912,"access_kind":"write","ppu_mode":3,"count":1,"first_seen":12,"last_seen":12}
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

## 8. GB CLI 仕様

### 8.1 analyze

```bash
sarakura gb analyze \
  --metadata out/kitaqgb_build_metadata.json \
  --events out/kokura_diagnostic_events.jsonl \
  --out out/sarakura_gb
```

Options:

| option | 必須 | 説明 |
|---|---:|---|
| `--metadata <path>` | yes | KITAQGBが出力したbuild metadata JSON |
| `--events <path>` | yes | KOKURAが出力したruntime diagnostic events JSONL |
| `--out <dir>` | yes | SARAKURA出力ディレクトリ |
| `--rom <path>` | no | repro bundleにhashだけ記録。`--include-rom`がない限り同梱しない |
| `--symbols <path>` | no | KITAQGB/KOKURA symbol map補助入力 |
| `--trace-dir <dir>` | no | trace window参照先 |
| `--snapshot-dir <dir>` | no | snapshot参照先 |
| `--min-confidence <float>` | no | repair_promptへ出す最低confidence。既定0.50 |
| `--strict-schema` | no | unknown fieldsをwarningではなくerrorにする |

### 8.2 output

```text
out/sarakura_gb/
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
## 9. GB側 全想定診断セット 48件

以下の48件をSARAKURA GB v1の全診断セットとする。

## 10. 診断一覧サマリ

| ID | Category | Primary event_type | Severity | Title |
|---|---|---|---|---|
| GB001 | VRAM / PPU access | `gb.vram.write.blocked` | `error` | VRAM write outside VBlank / safe LCD window |
| GB002 | VRAM / PPU access | `gb.vram.read.blocked` | `error` | VRAM read during blocked PPU mode |
| GB003 | VRAM / PPU access | `gb.vram.tile_upload.over_budget` | `error` | Tile data upload exceeds VBlank budget |
| GB004 | VRAM / PPU access | `gb.tilemap.write.render_collision` | `error` | Tilemap write collides with rendering |
| GB005 | VRAM / PPU access | `gb.cgb.attr.mismatch` | `warning` | CGB BG/window attribute metadata mismatch |
| GB006 | VRAM / PPU access | `gb.cgb.palette.write_unsafe` | `error` | CGB palette write outside safe window |
| GB007 | VRAM / PPU access | `gb.ppu.scroll.midframe_undeclared` | `warning` | Undeclared mid-frame scroll register write |
| GB008 | VBlank / LCDC / STAT | `gb.lcdc.toggle.unsafe` | `error` | Unsafe LCDC disable/enable sequence |
| GB009 | VBlank / LCDC / STAT | `gb.stat.wait.stall` | `warning` | STAT mode wait loop stalls frame progress |
| GB010 | VBlank / LCDC / STAT | `gb.present.unguarded` | `error` | PresentScreen without VBlank guard |
| GB011 | OAM / sprite / DMA | `gb.oam.write.blocked` | `error` | OAM write during mode 2/3 |
| GB012 | OAM / sprite / DMA | `gb.oam_dma.source.invalid` | `error` | OAM DMA source alignment or bank risk |
| GB013 | OAM / sprite / DMA | `gb.oam_dma.code_region.unsafe` | `error` | OAM DMA invoked from unsafe code region |
| GB014 | OAM / sprite / DMA | `gb.sprite.scanline.overflow` | `warning` | Sprite count overflow on one scanline |
| GB015 | OAM / sprite / DMA | `gb.metasprite.bounds.overflow` | `warning` | Metasprite coordinate/bounds overflow |
| GB016 | OAM / sprite / DMA | `gb.sprite.tile.out_of_range` | `error` | Sprite tile index out of loaded range |
| GB017 | OAM / sprite / DMA | `gb.shadow_oam.not_flushed` | `warning` | Shadow OAM not flushed before frame end |
| GB018 | HDMA / GDMA | `gb.hdma.start.not_hblank` | `error` | HDMA started outside HBlank mode |
| GB019 | HDMA / GDMA | `gb.hdma.length.overrun` | `error` | HDMA length overrun |
| GB020 | HDMA / GDMA | `gb.gdma.stall.runtime_impact` | `warning` | GDMA stall impacts audio or input |
| GB021 | HDMA / GDMA | `gb.hdma.dest.invalid` | `error` | HDMA VRAM destination invalid |
| GB022 | HDMA / GDMA | `gb.cgb_dma.bank_switch.conflict` | `error` | CGB DMA conflicts with bank switch |
| GB023 | VBlank / LCDC / STAT | `gb.vblank_queue.not_drained` | `warning` | VBlank queue not drained before render resumes |
| GB024 | VBlank / LCDC / STAT | `gb.waitvblank.reentrant` | `error` | WaitVBlank reentrant or nested use |
| GB025 | VBlank / LCDC / STAT | `gb.stat.lyc.mismatch` | `warning` | LY/LYC STAT IRQ configuration mismatch |
| GB026 | VBlank / LCDC / STAT | `gb.frame.budget_overrun` | `warning` | Frame budget overrun |
| GB027 | VBlank / LCDC / STAT | `gb.lcd_timing.assumption_broken` | `warning` | LCD mode timing assumption broken |
| GB028 | bank switch / farcall | `gb.rom_bank.restore_missing` | `error` | ROM bank switch without restore |
| GB029 | bank switch / farcall | `gb.farcall.bank_mismatch` | `error` | Farcall target bank mismatch |
| GB030 | bank switch / farcall | `gb.banked_func.direct_call` | `error` | Banked function called directly |
| GB031 | bank switch / farcall | `gb.asset.bank_mismatch` | `error` | Asset bank metadata mismatch |
| GB032 | bank switch / farcall | `gb.wram_bank.live_pointer` | `error` | WRAM bank switch with live pointers |
| GB033 | APU / audio timing | `gb.apu.write_storm` | `warning` | APU register write storm |
| GB034 | APU / audio timing | `gb.audio.tick.missed` | `warning` | Audio tick not called during CPU think loop |
| GB035 | APU / audio timing | `gb.apu.ch3.wave_write_while_on` | `error` | CH3 wave RAM write while channel is playing |
| GB036 | APU / audio timing | `gb.apu.nr52.toggle_unsafe` | `error` | NR52 power toggle unsafe |
| GB037 | APU / audio timing | `gb.bgm.pattern.out_of_range` | `error` | BGM pattern index out of range |
| GB038 | interrupt / stack / ABI | `gb.ime.leak` | `error` | IME state leak after critical section |
| GB039 | interrupt / stack / ABI | `gb.isr.stack_overflow_risk` | `warning` | ISR stack overflow risk |
| GB040 | interrupt / stack / ABI | `gb.abi.register_clobber` | `error` | Calling convention register clobber |
| GB041 | interrupt / stack / ABI | `gb.hram.overlap` | `error` | HRAM workarea overlap |
| GB042 | interrupt / stack / ABI | `gb.stack.reserve_violation` | `error` | Stack top/reserve violation |
| GB043 | asset / tilemap / sprite metadata | `gb.asset.font_tile_collision` | `warning` | Font tile range collision |
| GB044 | asset / tilemap / sprite metadata | `gb.tilemap.dimension_overflow` | `error` | Tilemap dimension overflow |
| GB045 | asset / tilemap / sprite metadata | `gb.asset.decompress.overflow` | `error` | Asset decompression overflows destination |
| GB046 | profiler / culprit ranking | `gb.profiler.culprit.hotspot` | `info` | Profiler culprit ranking identifies hot diagnostic site |
| GB047 | replay / snapshot / trace link | `gb.replay.divergence` | `error` | Replay divergence at diagnostic site |
| GB048 | replay / snapshot / trace link | `gb.trace.snapshot_missing` | `warning` | Snapshot or trace link missing |


### GB001 VRAM write outside VBlank / safe LCD window

| 項目 | 内容 |
|---|---|
| Category | VRAM / PPU access |
| Severity | `error` |
| Primary event_type | `gb.vram.write.blocked` |
| Primary operation_kind | `vram_write` |
| Fixture | `tests/fixtures/gb/gb001/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.vram.write.blocked` が出現する。
  - `pc` / bank / `operation_kind=vram_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `requires_vblank`, `vram_op_id`, `queue_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_mode`, `lcdc_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.vram.write.blocked` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vram_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vram_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `move-to-vblank-queue`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.vram.write.blocked` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.vram.write.blocked` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB001`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.vram.write.blocked` の正規化を追加する。
  2. `GB` metadata mapperで `vram_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB001` ruleを追加する。
  4. repair hint generatorへ `move-to-vblank-queue` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB002 VRAM read during blocked PPU mode

| 項目 | 内容 |
|---|---|
| Category | VRAM / PPU access |
| Severity | `error` |
| Primary event_type | `gb.vram.read.blocked` |
| Primary operation_kind | `vram_read` |
| Fixture | `tests/fixtures/gb/gb002/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.vram.read.blocked` が出現する。
  - `pc` / bank / `operation_kind=vram_read` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `vram_op_id`, `requires_vblank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_mode`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.vram.read.blocked` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vram_read` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vram_read` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `guard-with-wait-or-shadow-copy`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.vram.read.blocked` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.vram.read.blocked` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB002`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.vram.read.blocked` の正規化を追加する。
  2. `GB` metadata mapperで `vram_read` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB002` ruleを追加する。
  4. repair hint generatorへ `guard-with-wait-or-shadow-copy` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB003 Tile data upload exceeds VBlank budget

| 項目 | 内容 |
|---|---|
| Category | VRAM / PPU access |
| Severity | `error` |
| Primary event_type | `gb.vram.tile_upload.over_budget` |
| Primary operation_kind | `tile_upload` |
| Fixture | `tests/fixtures/gb/gb003/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.vram.tile_upload.over_budget` が出現する。
  - `pc` / bank / `operation_kind=tile_upload` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `asset_id`, `vram_op_id`, `queue_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `bytes`, `budget_cycles`, `actual_cycles`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.vram.tile_upload.over_budget` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `tile_upload` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `tile_upload` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `split-transfer-across-frames`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.vram.tile_upload.over_budget` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.vram.tile_upload.over_budget` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB003`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.vram.tile_upload.over_budget` の正規化を追加する。
  2. `GB` metadata mapperで `tile_upload` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB003` ruleを追加する。
  4. repair hint generatorへ `split-transfer-across-frames` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB004 Tilemap write collides with rendering

| 項目 | 内容 |
|---|---|
| Category | VRAM / PPU access |
| Severity | `error` |
| Primary event_type | `gb.tilemap.write.render_collision` |
| Primary operation_kind | `tilemap_write` |
| Fixture | `tests/fixtures/gb/gb004/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.tilemap.write.render_collision` が出現する。
  - `pc` / bank / `operation_kind=tilemap_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `tilemap_id`, `vram_op_id`, `requires_vblank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_mode`, `addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.tilemap.write.render_collision` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `tilemap_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `tilemap_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `defer-tilemap-write-to-vblank`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.tilemap.write.render_collision` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.tilemap.write.render_collision` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB004`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.tilemap.write.render_collision` の正規化を追加する。
  2. `GB` metadata mapperで `tilemap_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB004` ruleを追加する。
  4. repair hint generatorへ `defer-tilemap-write-to-vblank` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB005 CGB BG/window attribute metadata mismatch

| 項目 | 内容 |
|---|---|
| Category | VRAM / PPU access |
| Severity | `warning` |
| Primary event_type | `gb.cgb.attr.mismatch` |
| Primary operation_kind | `cgb_attr_write` |
| Fixture | `tests/fixtures/gb/gb005/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.cgb.attr.mismatch` が出現する。
  - `pc` / bank / `operation_kind=cgb_attr_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `tilemap_id`, `asset_id`, `section`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `cgb_vbk`, `attr_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.cgb.attr.mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `cgb_attr_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `cgb_attr_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-asset-attribute-metadata`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.cgb.attr.mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.cgb.attr.mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB005`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.cgb.attr.mismatch` の正規化を追加する。
  2. `GB` metadata mapperで `cgb_attr_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB005` ruleを追加する。
  4. repair hint generatorへ `fix-asset-attribute-metadata` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB006 CGB palette write outside safe window

| 項目 | 内容 |
|---|---|
| Category | VRAM / PPU access |
| Severity | `error` |
| Primary event_type | `gb.cgb.palette.write_unsafe` |
| Primary operation_kind | `palette_write` |
| Fixture | `tests/fixtures/gb/gb006/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.cgb.palette.write_unsafe` が出現する。
  - `pc` / bank / `operation_kind=palette_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `operation_kind`, `symbol_name`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `bcps`, `bcpd`, `ocps`, `ocpd`, `ppu_mode`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.cgb.palette.write_unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `palette_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `palette_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `queue-palette-update`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.cgb.palette.write_unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.cgb.palette.write_unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB006`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.cgb.palette.write_unsafe` の正規化を追加する。
  2. `GB` metadata mapperで `palette_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB006` ruleを追加する。
  4. repair hint generatorへ `queue-palette-update` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB007 Undeclared mid-frame scroll register write

| 項目 | 内容 |
|---|---|
| Category | VRAM / PPU access |
| Severity | `warning` |
| Primary event_type | `gb.ppu.scroll.midframe_undeclared` |
| Primary operation_kind | `scroll_write` |
| Fixture | `tests/fixtures/gb/gb007/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.ppu.scroll.midframe_undeclared` が出現する。
  - `pc` / bank / `operation_kind=scroll_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `operation_kind`, `symbol_name`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `scx`, `scy`, `wx`, `wy`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.ppu.scroll.midframe_undeclared` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `scroll_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `scroll_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `declare-raster-effect-or-move-to-vblank`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.ppu.scroll.midframe_undeclared` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.ppu.scroll.midframe_undeclared` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB007`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.ppu.scroll.midframe_undeclared` の正規化を追加する。
  2. `GB` metadata mapperで `scroll_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB007` ruleを追加する。
  4. repair hint generatorへ `declare-raster-effect-or-move-to-vblank` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB008 Unsafe LCDC disable/enable sequence

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `error` |
| Primary event_type | `gb.lcdc.toggle.unsafe` |
| Primary operation_kind | `lcdc_toggle` |
| Fixture | `tests/fixtures/gb/gb008/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.lcdc.toggle.unsafe` が出現する。
  - `pc` / bank / `operation_kind=lcdc_toggle` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `requires_vblank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `lcdc_state`, `ppu_mode`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.lcdc.toggle.unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `lcdc_toggle` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `lcdc_toggle` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `guard-lcdc-toggle-with-vblank-and-frame-boundary`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.lcdc.toggle.unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.lcdc.toggle.unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB008`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.lcdc.toggle.unsafe` の正規化を追加する。
  2. `GB` metadata mapperで `lcdc_toggle` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB008` ruleを追加する。
  4. repair hint generatorへ `guard-lcdc-toggle-with-vblank-and-frame-boundary` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB009 STAT mode wait loop stalls frame progress

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `warning` |
| Primary event_type | `gb.stat.wait.stall` |
| Primary operation_kind | `stat_wait` |
| Fixture | `tests/fixtures/gb/gb009/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.stat.wait.stall` が出現する。
  - `pc` / bank / `operation_kind=stat_wait` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `stat_state`, `loop_iterations`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.stat.wait.stall` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `stat_wait` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `stat_wait` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `replace-busy-wait-with-timeout-or-vblank-queue`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.stat.wait.stall` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.stat.wait.stall` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB009`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.stat.wait.stall` の正規化を追加する。
  2. `GB` metadata mapperで `stat_wait` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB009` ruleを追加する。
  4. repair hint generatorへ `replace-busy-wait-with-timeout-or-vblank-queue` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB010 PresentScreen without VBlank guard

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `error` |
| Primary event_type | `gb.present.unguarded` |
| Primary operation_kind | `present_screen` |
| Fixture | `tests/fixtures/gb/gb010/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.present.unguarded` が出現する。
  - `pc` / bank / `operation_kind=present_screen` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `queue_id`, `requires_vblank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_mode`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.present.unguarded` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `present_screen` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `present_screen` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `wrap-present-with-waitvblank-or-runtime-present-helper`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.present.unguarded` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.present.unguarded` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB010`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.present.unguarded` の正規化を追加する。
  2. `GB` metadata mapperで `present_screen` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB010` ruleを追加する。
  4. repair hint generatorへ `wrap-present-with-waitvblank-or-runtime-present-helper` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB011 OAM write during mode 2/3

| 項目 | 内容 |
|---|---|
| Category | OAM / sprite / DMA |
| Severity | `error` |
| Primary event_type | `gb.oam.write.blocked` |
| Primary operation_kind | `oam_write` |
| Fixture | `tests/fixtures/gb/gb011/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.oam.write.blocked` が出現する。
  - `pc` / bank / `operation_kind=oam_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `oam_op_id`, `sprite_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_mode`, `addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.oam.write.blocked` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `oam_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `oam_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `write-shadow-oam-then-dma`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.oam.write.blocked` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.oam.write.blocked` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB011`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.oam.write.blocked` の正規化を追加する。
  2. `GB` metadata mapperで `oam_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB011` ruleを追加する。
  4. repair hint generatorへ `write-shadow-oam-then-dma` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB012 OAM DMA source alignment or bank risk

| 項目 | 内容 |
|---|---|
| Category | OAM / sprite / DMA |
| Severity | `error` |
| Primary event_type | `gb.oam_dma.source.invalid` |
| Primary operation_kind | `oam_dma` |
| Fixture | `tests/fixtures/gb/gb012/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.oam_dma.source.invalid` が出現する。
  - `pc` / bank / `operation_kind=oam_dma` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dma_op_id`, `section`, `wram_bank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dma_state`, `source_addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.oam_dma.source.invalid` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `oam_dma` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `oam_dma` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `move-oam-buffer-to-fixed-aligned-wram`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.oam_dma.source.invalid` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.oam_dma.source.invalid` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB012`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.oam_dma.source.invalid` の正規化を追加する。
  2. `GB` metadata mapperで `oam_dma` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB012` ruleを追加する。
  4. repair hint generatorへ `move-oam-buffer-to-fixed-aligned-wram` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB013 OAM DMA invoked from unsafe code region

| 項目 | 内容 |
|---|---|
| Category | OAM / sprite / DMA |
| Severity | `error` |
| Primary event_type | `gb.oam_dma.code_region.unsafe` |
| Primary operation_kind | `oam_dma` |
| Fixture | `tests/fixtures/gb/gb013/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.oam_dma.code_region.unsafe` が出現する。
  - `pc` / bank / `operation_kind=oam_dma` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dma_op_id`, `section`, `hram_usage`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dma_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.oam_dma.code_region.unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `oam_dma` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `oam_dma` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `use-hram-oam-dma-stub`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.oam_dma.code_region.unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.oam_dma.code_region.unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB013`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.oam_dma.code_region.unsafe` の正規化を追加する。
  2. `GB` metadata mapperで `oam_dma` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB013` ruleを追加する。
  4. repair hint generatorへ `use-hram-oam-dma-stub` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB014 Sprite count overflow on one scanline

| 項目 | 内容 |
|---|---|
| Category | OAM / sprite / DMA |
| Severity | `warning` |
| Primary event_type | `gb.sprite.scanline.overflow` |
| Primary operation_kind | `sprite_eval` |
| Fixture | `tests/fixtures/gb/gb014/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.sprite.scanline.overflow` が出現する。
  - `pc` / bank / `operation_kind=sprite_eval` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `sprite_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sprite_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.sprite.scanline.overflow` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `sprite_eval` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `sprite_eval` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `reduce-sprites-or-sort-oam-priority`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.sprite.scanline.overflow` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.sprite.scanline.overflow` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB014`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.sprite.scanline.overflow` の正規化を追加する。
  2. `GB` metadata mapperで `sprite_eval` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB014` ruleを追加する。
  4. repair hint generatorへ `reduce-sprites-or-sort-oam-priority` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB015 Metasprite coordinate/bounds overflow

| 項目 | 内容 |
|---|---|
| Category | OAM / sprite / DMA |
| Severity | `warning` |
| Primary event_type | `gb.metasprite.bounds.overflow` |
| Primary operation_kind | `metasprite_draw` |
| Fixture | `tests/fixtures/gb/gb015/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.metasprite.bounds.overflow` が出現する。
  - `pc` / bank / `operation_kind=metasprite_draw` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `sprite_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `x`, `y`, `width`, `height`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.metasprite.bounds.overflow` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `metasprite_draw` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `metasprite_draw` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `clip-metasprite-before-oam-write`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.metasprite.bounds.overflow` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.metasprite.bounds.overflow` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB015`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.metasprite.bounds.overflow` の正規化を追加する。
  2. `GB` metadata mapperで `metasprite_draw` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB015` ruleを追加する。
  4. repair hint generatorへ `clip-metasprite-before-oam-write` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB016 Sprite tile index out of loaded range

| 項目 | 内容 |
|---|---|
| Category | OAM / sprite / DMA |
| Severity | `error` |
| Primary event_type | `gb.sprite.tile.out_of_range` |
| Primary operation_kind | `sprite_tile` |
| Fixture | `tests/fixtures/gb/gb016/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.sprite.tile.out_of_range` が出現する。
  - `pc` / bank / `operation_kind=sprite_tile` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `sprite_id`, `asset_id`, `section`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `tile_index`, `loaded_range`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.sprite.tile.out_of_range` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `sprite_tile` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `sprite_tile` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-sprite-tile-base-or-asset-bank`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.sprite.tile.out_of_range` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.sprite.tile.out_of_range` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB016`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.sprite.tile.out_of_range` の正規化を追加する。
  2. `GB` metadata mapperで `sprite_tile` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB016` ruleを追加する。
  4. repair hint generatorへ `fix-sprite-tile-base-or-asset-bank` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB017 Shadow OAM not flushed before frame end

| 項目 | 内容 |
|---|---|
| Category | OAM / sprite / DMA |
| Severity | `warning` |
| Primary event_type | `gb.shadow_oam.not_flushed` |
| Primary operation_kind | `shadow_oam` |
| Fixture | `tests/fixtures/gb/gb017/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.shadow_oam.not_flushed` が出現する。
  - `pc` / bank / `operation_kind=shadow_oam` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `oam_op_id`, `queue_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `last_oam_dma_frame`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.shadow_oam.not_flushed` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `shadow_oam` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `shadow_oam` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `ensure-oam-dma-once-per-frame`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.shadow_oam.not_flushed` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.shadow_oam.not_flushed` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB017`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.shadow_oam.not_flushed` の正規化を追加する。
  2. `GB` metadata mapperで `shadow_oam` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB017` ruleを追加する。
  4. repair hint generatorへ `ensure-oam-dma-once-per-frame` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB018 HDMA started outside HBlank mode

| 項目 | 内容 |
|---|---|
| Category | HDMA / GDMA |
| Severity | `error` |
| Primary event_type | `gb.hdma.start.not_hblank` |
| Primary operation_kind | `hdma_start` |
| Fixture | `tests/fixtures/gb/gb018/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.hdma.start.not_hblank` が出現する。
  - `pc` / bank / `operation_kind=hdma_start` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dma_op_id`, `vram_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ppu_mode`, `hdma_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.hdma.start.not_hblank` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `hdma_start` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `hdma_start` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `start-hdma-only-from-hblank-safe-helper`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.hdma.start.not_hblank` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.hdma.start.not_hblank` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB018`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.hdma.start.not_hblank` の正規化を追加する。
  2. `GB` metadata mapperで `hdma_start` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB018` ruleを追加する。
  4. repair hint generatorへ `start-hdma-only-from-hblank-safe-helper` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB019 HDMA length overrun

| 項目 | 内容 |
|---|---|
| Category | HDMA / GDMA |
| Severity | `error` |
| Primary event_type | `gb.hdma.length.overrun` |
| Primary operation_kind | `hdma_transfer` |
| Fixture | `tests/fixtures/gb/gb019/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.hdma.length.overrun` が出現する。
  - `pc` / bank / `operation_kind=hdma_transfer` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dma_op_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `length`, `remaining`, `dest_addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.hdma.length.overrun` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `hdma_transfer` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `hdma_transfer` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `clamp-or-split-hdma-length`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.hdma.length.overrun` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.hdma.length.overrun` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB019`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.hdma.length.overrun` の正規化を追加する。
  2. `GB` metadata mapperで `hdma_transfer` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB019` ruleを追加する。
  4. repair hint generatorへ `clamp-or-split-hdma-length` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB020 GDMA stall impacts audio or input

| 項目 | 内容 |
|---|---|
| Category | HDMA / GDMA |
| Severity | `warning` |
| Primary event_type | `gb.gdma.stall.runtime_impact` |
| Primary operation_kind | `gdma_transfer` |
| Fixture | `tests/fixtures/gb/gb020/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.gdma.stall.runtime_impact` が出現する。
  - `pc` / bank / `operation_kind=gdma_transfer` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dma_op_id`, `apu_update_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `stall_cycles`, `missed_audio_ticks`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.gdma.stall.runtime_impact` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `gdma_transfer` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `gdma_transfer` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `prefer-hdma-or-split-gdma`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.gdma.stall.runtime_impact` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.gdma.stall.runtime_impact` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB020`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.gdma.stall.runtime_impact` の正規化を追加する。
  2. `GB` metadata mapperで `gdma_transfer` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB020` ruleを追加する。
  4. repair hint generatorへ `prefer-hdma-or-split-gdma` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB021 HDMA VRAM destination invalid

| 項目 | 内容 |
|---|---|
| Category | HDMA / GDMA |
| Severity | `error` |
| Primary event_type | `gb.hdma.dest.invalid` |
| Primary operation_kind | `hdma_transfer` |
| Fixture | `tests/fixtures/gb/gb021/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.hdma.dest.invalid` が出現する。
  - `pc` / bank / `operation_kind=hdma_transfer` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dma_op_id`, `vram_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dest_addr`, `length`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.hdma.dest.invalid` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `hdma_transfer` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `hdma_transfer` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `validate-hdma-destination-range`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.hdma.dest.invalid` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.hdma.dest.invalid` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB021`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.hdma.dest.invalid` の正規化を追加する。
  2. `GB` metadata mapperで `hdma_transfer` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB021` ruleを追加する。
  4. repair hint generatorへ `validate-hdma-destination-range` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB022 CGB DMA conflicts with bank switch

| 項目 | 内容 |
|---|---|
| Category | HDMA / GDMA |
| Severity | `error` |
| Primary event_type | `gb.cgb_dma.bank_switch.conflict` |
| Primary operation_kind | `dma_bank_switch` |
| Fixture | `tests/fixtures/gb/gb022/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.cgb_dma.bank_switch.conflict` が出現する。
  - `pc` / bank / `operation_kind=dma_bank_switch` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `dma_op_id`, `bank_switch_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dma_state`, `rom_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.cgb_dma.bank_switch.conflict` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `dma_bank_switch` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `dma_bank_switch` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `pin-bank-during-dma-or-copy-to-wram`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.cgb_dma.bank_switch.conflict` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.cgb_dma.bank_switch.conflict` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB022`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.cgb_dma.bank_switch.conflict` の正規化を追加する。
  2. `GB` metadata mapperで `dma_bank_switch` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB022` ruleを追加する。
  4. repair hint generatorへ `pin-bank-during-dma-or-copy-to-wram` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB023 VBlank queue not drained before render resumes

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `warning` |
| Primary event_type | `gb.vblank_queue.not_drained` |
| Primary operation_kind | `vblank_queue` |
| Fixture | `tests/fixtures/gb/gb023/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.vblank_queue.not_drained` が出現する。
  - `pc` / bank / `operation_kind=vblank_queue` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `queue_id`, `vram_op_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `queue_len`, `budget_cycles`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.vblank_queue.not_drained` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `vblank_queue` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `vblank_queue` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `prioritize-or-split-vblank-queue`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.vblank_queue.not_drained` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.vblank_queue.not_drained` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB023`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.vblank_queue.not_drained` の正規化を追加する。
  2. `GB` metadata mapperで `vblank_queue` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB023` ruleを追加する。
  4. repair hint generatorへ `prioritize-or-split-vblank-queue` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB024 WaitVBlank reentrant or nested use

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `error` |
| Primary event_type | `gb.waitvblank.reentrant` |
| Primary operation_kind | `waitvblank` |
| Fixture | `tests/fixtures/gb/gb024/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.waitvblank.reentrant` が出現する。
  - `pc` / bank / `operation_kind=waitvblank` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `call_depth`, `ime_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.waitvblank.reentrant` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `waitvblank` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `waitvblank` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `remove-nested-wait-or-use-nonblocking-state`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.waitvblank.reentrant` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.waitvblank.reentrant` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB024`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.waitvblank.reentrant` の正規化を追加する。
  2. `GB` metadata mapperで `waitvblank` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB024` ruleを追加する。
  4. repair hint generatorへ `remove-nested-wait-or-use-nonblocking-state` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB025 LY/LYC STAT IRQ configuration mismatch

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `warning` |
| Primary event_type | `gb.stat.lyc.mismatch` |
| Primary operation_kind | `stat_irq` |
| Fixture | `tests/fixtures/gb/gb025/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.stat.lyc.mismatch` が出現する。
  - `pc` / bank / `operation_kind=stat_irq` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `operation_kind`, `symbol_name`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ly`, `lyc`, `stat_state`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.stat.lyc.mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `stat_irq` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `stat_irq` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `synchronize-lyc-and-stat-mask`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.stat.lyc.mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.stat.lyc.mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB025`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.stat.lyc.mismatch` の正規化を追加する。
  2. `GB` metadata mapperで `stat_irq` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB025` ruleを追加する。
  4. repair hint generatorへ `synchronize-lyc-and-stat-mask` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB026 Frame budget overrun

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `warning` |
| Primary event_type | `gb.frame.budget_overrun` |
| Primary operation_kind | `frame_runtime` |
| Fixture | `tests/fixtures/gb/gb026/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.frame.budget_overrun` が出現する。
  - `pc` / bank / `operation_kind=frame_runtime` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `queue_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `frame_cycles`, `budget_cycles`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.frame.budget_overrun` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `frame_runtime` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `frame_runtime` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `move-heavy-work-to-incremental-state-machine`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.frame.budget_overrun` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.frame.budget_overrun` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB026`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.frame.budget_overrun` の正規化を追加する。
  2. `GB` metadata mapperで `frame_runtime` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB026` ruleを追加する。
  4. repair hint generatorへ `move-heavy-work-to-incremental-state-machine` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB027 LCD mode timing assumption broken

| 項目 | 内容 |
|---|---|
| Category | VBlank / LCDC / STAT |
| Severity | `warning` |
| Primary event_type | `gb.lcd_timing.assumption_broken` |
| Primary operation_kind | `timing_assumption` |
| Fixture | `tests/fixtures/gb/gb027/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.lcd_timing.assumption_broken` が出現する。
  - `pc` / bank / `operation_kind=timing_assumption` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `build_warning_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `expected_mode`, `actual_mode`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.lcd_timing.assumption_broken` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `timing_assumption` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `timing_assumption` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `replace-hardcoded-delay-with-mode-aware-helper`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.lcd_timing.assumption_broken` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.lcd_timing.assumption_broken` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB027`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.lcd_timing.assumption_broken` の正規化を追加する。
  2. `GB` metadata mapperで `timing_assumption` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB027` ruleを追加する。
  4. repair hint generatorへ `replace-hardcoded-delay-with-mode-aware-helper` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB028 ROM bank switch without restore

| 項目 | 内容 |
|---|---|
| Category | bank switch / farcall |
| Severity | `error` |
| Primary event_type | `gb.rom_bank.restore_missing` |
| Primary operation_kind | `bank_switch` |
| Fixture | `tests/fixtures/gb/gb028/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.rom_bank.restore_missing` が出現する。
  - `pc` / bank / `operation_kind=bank_switch` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `bank_switch_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `rom_bank_before`, `rom_bank_after`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.rom_bank.restore_missing` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `bank_switch` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `bank_switch` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `restore-bank-in-finally-style-guard`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.rom_bank.restore_missing` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.rom_bank.restore_missing` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB028`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.rom_bank.restore_missing` の正規化を追加する。
  2. `GB` metadata mapperで `bank_switch` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB028` ruleを追加する。
  4. repair hint generatorへ `restore-bank-in-finally-style-guard` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB029 Farcall target bank mismatch

| 項目 | 内容 |
|---|---|
| Category | bank switch / farcall |
| Severity | `error` |
| Primary event_type | `gb.farcall.bank_mismatch` |
| Primary operation_kind | `farcall` |
| Fixture | `tests/fixtures/gb/gb029/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.farcall.bank_mismatch` が出現する。
  - `pc` / bank / `operation_kind=farcall` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `farcall_id`, `rom_bank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `target_bank`, `actual_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.farcall.bank_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `farcall` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `farcall` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-farcall-metadata-or-call-site`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.farcall.bank_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.farcall.bank_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB029`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.farcall.bank_mismatch` の正規化を追加する。
  2. `GB` metadata mapperで `farcall` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB029` ruleを追加する。
  4. repair hint generatorへ `fix-farcall-metadata-or-call-site` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB030 Banked function called directly

| 項目 | 内容 |
|---|---|
| Category | bank switch / farcall |
| Severity | `error` |
| Primary event_type | `gb.banked_func.direct_call` |
| Primary operation_kind | `call` |
| Fixture | `tests/fixtures/gb/gb030/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.banked_func.direct_call` が出現する。
  - `pc` / bank / `operation_kind=call` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `rom_bank`, `farcall_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `target_pc`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.banked_func.direct_call` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `call` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `call` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `replace-direct-call-with-farcall-wrapper`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.banked_func.direct_call` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.banked_func.direct_call` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB030`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.banked_func.direct_call` の正規化を追加する。
  2. `GB` metadata mapperで `call` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB030` ruleを追加する。
  4. repair hint generatorへ `replace-direct-call-with-farcall-wrapper` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB031 Asset bank metadata mismatch

| 項目 | 内容 |
|---|---|
| Category | bank switch / farcall |
| Severity | `error` |
| Primary event_type | `gb.asset.bank_mismatch` |
| Primary operation_kind | `asset_load` |
| Fixture | `tests/fixtures/gb/gb031/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.asset.bank_mismatch` が出現する。
  - `pc` / bank / `operation_kind=asset_load` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `asset_id`, `rom_bank`, `section`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `expected_bank`, `actual_bank`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.asset.bank_mismatch` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `asset_load` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `asset_load` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-asset-bank-table-or-loader`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.asset.bank_mismatch` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.asset.bank_mismatch` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB031`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.asset.bank_mismatch` の正規化を追加する。
  2. `GB` metadata mapperで `asset_load` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB031` ruleを追加する。
  4. repair hint generatorへ `fix-asset-bank-table-or-loader` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB032 WRAM bank switch with live pointers

| 項目 | 内容 |
|---|---|
| Category | bank switch / farcall |
| Severity | `error` |
| Primary event_type | `gb.wram_bank.live_pointer` |
| Primary operation_kind | `wram_bank_switch` |
| Fixture | `tests/fixtures/gb/gb032/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.wram_bank.live_pointer` が出現する。
  - `pc` / bank / `operation_kind=wram_bank_switch` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `wram_bank`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `old_wram_bank`, `new_wram_bank`, `live_pointer_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.wram_bank.live_pointer` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `wram_bank_switch` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `wram_bank_switch` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `avoid-switch-or-copy-through-fixed-buffer`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.wram_bank.live_pointer` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.wram_bank.live_pointer` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB032`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.wram_bank.live_pointer` の正規化を追加する。
  2. `GB` metadata mapperで `wram_bank_switch` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB032` ruleを追加する。
  4. repair hint generatorへ `avoid-switch-or-copy-through-fixed-buffer` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB033 APU register write storm

| 項目 | 内容 |
|---|---|
| Category | APU / audio timing |
| Severity | `warning` |
| Primary event_type | `gb.apu.write_storm` |
| Primary operation_kind | `apu_write` |
| Fixture | `tests/fixtures/gb/gb033/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.apu.write_storm` が出現する。
  - `pc` / bank / `operation_kind=apu_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `apu_update_id`, `bgm_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `apu_state`, `write_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.apu.write_storm` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `apu_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `apu_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `rate-limit-audio-register-writes`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.apu.write_storm` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.apu.write_storm` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB033`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.apu.write_storm` の正規化を追加する。
  2. `GB` metadata mapperで `apu_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB033` ruleを追加する。
  4. repair hint generatorへ `rate-limit-audio-register-writes` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB034 Audio tick not called during CPU think loop

| 項目 | 内容 |
|---|---|
| Category | APU / audio timing |
| Severity | `warning` |
| Primary event_type | `gb.audio.tick.missed` |
| Primary operation_kind | `audio_tick` |
| Fixture | `tests/fixtures/gb/gb034/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.audio.tick.missed` が出現する。
  - `pc` / bank / `operation_kind=audio_tick` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `apu_update_id`, `bgm_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `missed_ticks`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.audio.tick.missed` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `audio_tick` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `audio_tick` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `call-audio-tick-inside-long-running-search-loop`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.audio.tick.missed` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.audio.tick.missed` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB034`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.audio.tick.missed` の正規化を追加する。
  2. `GB` metadata mapperで `audio_tick` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB034` ruleを追加する。
  4. repair hint generatorへ `call-audio-tick-inside-long-running-search-loop` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB035 CH3 wave RAM write while channel is playing

| 項目 | 内容 |
|---|---|
| Category | APU / audio timing |
| Severity | `error` |
| Primary event_type | `gb.apu.ch3.wave_write_while_on` |
| Primary operation_kind | `ch3_wave_write` |
| Fixture | `tests/fixtures/gb/gb035/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.apu.ch3.wave_write_while_on` が出現する。
  - `pc` / bank / `operation_kind=ch3_wave_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `apu_update_id`, `bgm_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `nr30`, `wave_addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.apu.ch3.wave_write_while_on` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `ch3_wave_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `ch3_wave_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `disable-ch3-before-wave-ram-update`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.apu.ch3.wave_write_while_on` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.apu.ch3.wave_write_while_on` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB035`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.apu.ch3.wave_write_while_on` の正規化を追加する。
  2. `GB` metadata mapperで `ch3_wave_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB035` ruleを追加する。
  4. repair hint generatorへ `disable-ch3-before-wave-ram-update` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB036 NR52 power toggle unsafe

| 項目 | 内容 |
|---|---|
| Category | APU / audio timing |
| Severity | `error` |
| Primary event_type | `gb.apu.nr52.toggle_unsafe` |
| Primary operation_kind | `apu_power` |
| Fixture | `tests/fixtures/gb/gb036/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.apu.nr52.toggle_unsafe` が出現する。
  - `pc` / bank / `operation_kind=apu_power` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `apu_update_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `nr52_before`, `nr52_after`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.apu.nr52.toggle_unsafe` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `apu_power` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `apu_power` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `centralize-apu-power-init-and-avoid-runtime-toggle`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.apu.nr52.toggle_unsafe` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.apu.nr52.toggle_unsafe` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB036`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.apu.nr52.toggle_unsafe` の正規化を追加する。
  2. `GB` metadata mapperで `apu_power` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB036` ruleを追加する。
  4. repair hint generatorへ `centralize-apu-power-init-and-avoid-runtime-toggle` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB037 BGM pattern index out of range

| 項目 | 内容 |
|---|---|
| Category | APU / audio timing |
| Severity | `error` |
| Primary event_type | `gb.bgm.pattern.out_of_range` |
| Primary operation_kind | `bgm_decode` |
| Fixture | `tests/fixtures/gb/gb037/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.bgm.pattern.out_of_range` が出現する。
  - `pc` / bank / `operation_kind=bgm_decode` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `bgm_id`, `apu_update_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `pattern_index`, `pattern_count`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.bgm.pattern.out_of_range` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `bgm_decode` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `bgm_decode` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-bgm-sequence-bounds-or-exporter-metadata`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.bgm.pattern.out_of_range` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.bgm.pattern.out_of_range` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB037`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.bgm.pattern.out_of_range` の正規化を追加する。
  2. `GB` metadata mapperで `bgm_decode` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB037` ruleを追加する。
  4. repair hint generatorへ `fix-bgm-sequence-bounds-or-exporter-metadata` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB038 IME state leak after critical section

| 項目 | 内容 |
|---|---|
| Category | interrupt / stack / ABI |
| Severity | `error` |
| Primary event_type | `gb.ime.leak` |
| Primary operation_kind | `interrupt_guard` |
| Fixture | `tests/fixtures/gb/gb038/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.ime.leak` が出現する。
  - `pc` / bank / `operation_kind=interrupt_guard` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `intrinsic_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `ime_before`, `ime_after`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.ime.leak` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `interrupt_guard` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `interrupt_guard` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `restore-ime-state-on-all-exit-paths`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.ime.leak` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.ime.leak` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB038`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.ime.leak` の正規化を追加する。
  2. `GB` metadata mapperで `interrupt_guard` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB038` ruleを追加する。
  4. repair hint generatorへ `restore-ime-state-on-all-exit-paths` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB039 ISR stack overflow risk

| 項目 | 内容 |
|---|---|
| Category | interrupt / stack / ABI |
| Severity | `warning` |
| Primary event_type | `gb.isr.stack_overflow_risk` |
| Primary operation_kind | `isr_stack` |
| Fixture | `tests/fixtures/gb/gb039/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.isr.stack_overflow_risk` が出現する。
  - `pc` / bank / `operation_kind=isr_stack` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `abi.sp_expected`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sp_min`, `stack_reserve`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.isr.stack_overflow_risk` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `isr_stack` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `isr_stack` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `reduce-isr-stack-use-or-raise-stack-reserve`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.isr.stack_overflow_risk` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.isr.stack_overflow_risk` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB039`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.isr.stack_overflow_risk` の正規化を追加する。
  2. `GB` metadata mapperで `isr_stack` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB039` ruleを追加する。
  4. repair hint generatorへ `reduce-isr-stack-use-or-raise-stack-reserve` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB040 Calling convention register clobber

| 項目 | 内容 |
|---|---|
| Category | interrupt / stack / ABI |
| Severity | `error` |
| Primary event_type | `gb.abi.register_clobber` |
| Primary operation_kind | `abi_check` |
| Fixture | `tests/fixtures/gb/gb040/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.abi.register_clobber` が出現する。
  - `pc` / bank / `operation_kind=abi_check` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `abi.register_preserve`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `register`, `expected`, `actual`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.abi.register_clobber` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `abi_check` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `abi_check` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
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
  - 同一ROM操作を再実行し、同じ `gb.abi.register_clobber` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.abi.register_clobber` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB040`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.abi.register_clobber` の正規化を追加する。
  2. `GB` metadata mapperで `abi_check` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB040` ruleを追加する。
  4. repair hint generatorへ `preserve-register-or-fix-abi-metadata` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB041 HRAM workarea overlap

| 項目 | 内容 |
|---|---|
| Category | interrupt / stack / ABI |
| Severity | `error` |
| Primary event_type | `gb.hram.overlap` |
| Primary operation_kind | `hram_alloc` |
| Fixture | `tests/fixtures/gb/gb041/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.hram.overlap` が出現する。
  - `pc` / bank / `operation_kind=hram_alloc` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `hram_usage`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `addr`, `owner_a`, `owner_b`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.hram.overlap` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `hram_alloc` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `hram_alloc` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `relocate-hram-symbol-or-reduce-stub-size`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.hram.overlap` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.hram.overlap` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB041`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.hram.overlap` の正規化を追加する。
  2. `GB` metadata mapperで `hram_alloc` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB041` ruleを追加する。
  4. repair hint generatorへ `relocate-hram-symbol-or-reduce-stub-size` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB042 Stack top/reserve violation

| 項目 | 内容 |
|---|---|
| Category | interrupt / stack / ABI |
| Severity | `error` |
| Primary event_type | `gb.stack.reserve_violation` |
| Primary operation_kind | `stack_usage` |
| Fixture | `tests/fixtures/gb/gb042/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.stack.reserve_violation` が出現する。
  - `pc` / bank / `operation_kind=stack_usage` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `abi.sp_expected`, `build_warning_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sp_min`, `reserve_bytes`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.stack.reserve_violation` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `stack_usage` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `stack_usage` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `adjust-stack-top-or-reduce-recursion/local-buffer`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.stack.reserve_violation` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.stack.reserve_violation` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB042`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.stack.reserve_violation` の正規化を追加する。
  2. `GB` metadata mapperで `stack_usage` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB042` ruleを追加する。
  4. repair hint generatorへ `adjust-stack-top-or-reduce-recursion/local-buffer` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB043 Font tile range collision

| 項目 | 内容 |
|---|---|
| Category | asset / tilemap / sprite metadata |
| Severity | `warning` |
| Primary event_type | `gb.asset.font_tile_collision` |
| Primary operation_kind | `asset_layout` |
| Fixture | `tests/fixtures/gb/gb043/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.asset.font_tile_collision` が出現する。
  - `pc` / bank / `operation_kind=asset_layout` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `asset_id`, `tilemap_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `tile_range_a`, `tile_range_b`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.asset.font_tile_collision` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `asset_layout` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `asset_layout` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `reassign-font-or-bg-tile-range`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.asset.font_tile_collision` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.asset.font_tile_collision` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB043`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.asset.font_tile_collision` の正規化を追加する。
  2. `GB` metadata mapperで `asset_layout` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB043` ruleを追加する。
  4. repair hint generatorへ `reassign-font-or-bg-tile-range` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB044 Tilemap dimension overflow

| 項目 | 内容 |
|---|---|
| Category | asset / tilemap / sprite metadata |
| Severity | `error` |
| Primary event_type | `gb.tilemap.dimension_overflow` |
| Primary operation_kind | `tilemap_write` |
| Fixture | `tests/fixtures/gb/gb044/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.tilemap.dimension_overflow` が出現する。
  - `pc` / bank / `operation_kind=tilemap_write` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `tilemap_id`, `asset_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `width`, `height`, `dest_addr`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.tilemap.dimension_overflow` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `tilemap_write` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `tilemap_write` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `clip-tilemap-or-fix-dimension-metadata`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.tilemap.dimension_overflow` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.tilemap.dimension_overflow` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB044`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.tilemap.dimension_overflow` の正規化を追加する。
  2. `GB` metadata mapperで `tilemap_write` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB044` ruleを追加する。
  4. repair hint generatorへ `clip-tilemap-or-fix-dimension-metadata` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB045 Asset decompression overflows destination

| 項目 | 内容 |
|---|---|
| Category | asset / tilemap / sprite metadata |
| Severity | `error` |
| Primary event_type | `gb.asset.decompress.overflow` |
| Primary operation_kind | `asset_decode` |
| Fixture | `tests/fixtures/gb/gb045/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.asset.decompress.overflow` が出現する。
  - `pc` / bank / `operation_kind=asset_decode` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `asset_id`, `section`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `dest_start`, `dest_end`, `decoded_size`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.asset.decompress.overflow` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `asset_decode` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `asset_decode` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `fix-compressed-size-or-destination-buffer`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.asset.decompress.overflow` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.asset.decompress.overflow` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB045`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.asset.decompress.overflow` の正規化を追加する。
  2. `GB` metadata mapperで `asset_decode` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB045` ruleを追加する。
  4. repair hint generatorへ `fix-compressed-size-or-destination-buffer` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB046 Profiler culprit ranking identifies hot diagnostic site

| 項目 | 内容 |
|---|---|
| Category | profiler / culprit ranking |
| Severity | `info` |
| Primary event_type | `gb.profiler.culprit.hotspot` |
| Primary operation_kind | `profiling` |
| Fixture | `tests/fixtures/gb/gb046/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.profiler.culprit.hotspot` が出現する。
  - `pc` / bank / `operation_kind=profiling` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `queue_id`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `sample_count`, `exclusive_cycles`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.profiler.culprit.hotspot` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `profiling` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `profiling` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `surface-hotspot-before-suggesting-code-change`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.profiler.culprit.hotspot` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.profiler.culprit.hotspot` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB046`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.profiler.culprit.hotspot` の正規化を追加する。
  2. `GB` metadata mapperで `profiling` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB046` ruleを追加する。
  4. repair hint generatorへ `surface-hotspot-before-suggesting-code-change` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB047 Replay divergence at diagnostic site

| 項目 | 内容 |
|---|---|
| Category | replay / snapshot / trace link |
| Severity | `error` |
| Primary event_type | `gb.replay.divergence` |
| Primary operation_kind | `replay` |
| Fixture | `tests/fixtures/gb/gb047/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.replay.divergence` が出現する。
  - `pc` / bank / `operation_kind=replay` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `replay_step`, `expected_hash`, `actual_hash`, `count`, `first_seen`, `last_seen`, `snapshot_ref`, `trace_window_ref`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.replay.divergence` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `replay` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `replay` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `attach-trace-window-and-minimize-nondeterminism`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.replay.divergence` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.replay.divergence` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB047`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.replay.divergence` の正規化を追加する。
  2. `GB` metadata mapperで `replay` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB047` ruleを追加する。
  4. repair hint generatorへ `attach-trace-window-and-minimize-nondeterminism` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

### GB048 Snapshot or trace link missing

| 項目 | 内容 |
|---|---|
| Category | replay / snapshot / trace link |
| Severity | `warning` |
| Primary event_type | `gb.trace.snapshot_missing` |
| Primary operation_kind | `trace_link` |
| Fixture | `tests/fixtures/gb/gb048/` |

- 検出条件:
  - `diagnostic_events.jsonl` に `gb.trace.snapshot_missing` が出現する。
  - `pc` / bank / `operation_kind=trace_link` を build metadata の `pc_range`、`function_id`、`intrinsic_id`、`asset_id`、またはqueue/mapper/audio操作IDへ対応付ける。
  - 同一bucketで `count > 0` かつ severity が `warning` 以上の場合にAI診断へ昇格する。`info`診断はHTML reportとculprit rankingへ出す。
- 必要メタデータ:
  - `source_file`, `source_line`, `function_id`, `function_name`, `symbol_name`, `operation_kind`, `pc_range`
- `diagnostic_events.jsonl` fields:
  - `event_id`, `event_type`, `severity`, `frame`, `scanline`, `dot`, `pc`, `snapshot_ref`, `trace_window_ref`, `count`, `first_seen`, `last_seen`
- `ai_diagnostics` fields:
  - `diagnostic_id`, `title`, `category`, `severity`, `summary`, `detected_condition`, `event_refs`, `metadata_refs`, `evidence_chain`, `target_candidates`, `repair_target`, `safe_patch_hint`, `confidence`, `retest_condition`, `fixture`, `codex_tasks`
- `evidence_chain`:
  1. runtime event `gb.trace.snapshot_missing` をevent_refとして記録する。
  2. `pc` と bank を `pc_range` へ照合し、関数・source line・symbolを特定する。
  3. `trace_link` に対応するmetadata idを探索し、該当するintrinsic / asset / queue / mapper / audio操作を結び付ける。
  4. `snapshot_ref` と `trace_window_ref` があれば、発生直前と発生時の状態をreportへリンクする。
- `target_candidates`:
  - Rank 1: `function_id` + `source_file:source_line` が一致する最小関数。
  - Rank 2: `trace_link` に対応する intrinsic/helper/queue/asset/mapper/audio operation。
  - Rank 3: 同一frame内で直前に実行された呼び出し元、または同一asset_idを共有する初期化処理。
- `repair_target`:
  - `primary_kind`: `source_line|intrinsic_call|runtime_queue|asset_metadata|bank_guard|audio_update` のいずれか。
  - `preferred_edit_scope`: `smallest-safe-change`。公開ABI、ROM/PRG bank layout、asset binary layoutは必要がない限り変更しない。
- `safe_patch_hint`:
  - `strategy`: `emit-snapshot-and-trace-window-for-this-event`。
  - `allowed_change_scope`: `source`, `metadata`, `runtime-helper`。mapper/FDS/HDMA等は専用helper修正を優先する。
  - `must_not_change`: unrelated gameplay logic, public ABI, unrelated bank layout。
- `confidence`:
  - 0.95: event、PC/bank、metadata id、snapshot、traceがすべて一致。
  - 0.85: event、PC/bank、function/source lineが一致。
  - 0.65: eventは明確だがmetadata idが欠落。
  - 0.40: symptoms only。HTML reportには出すが自動修正候補にはしない。
- `retest_condition`:
  - 同一ROM操作を再実行し、同じ `gb.trace.snapshot_missing` が同一PC/bankで再発しないこと。
  - `minimum_replay_frames`: 600。タイミング系は該当画面・該当演出まで到達するfixture replayを使う。
  - before/after比較で `resolved` または `improved` になること。`regressed` は失敗扱い。
- テストfixture:
  - `build_metadata.json`: 上記metadata fieldsを最小構成で含める。
  - `diagnostic_events.jsonl`: `gb.trace.snapshot_missing` を1件以上含める。
  - `expected_ai_diagnostics.json`: `GB048`、repair_target、safe_patch_hint、confidence範囲を検証する。
  - `README.md`: なぜこの診断が出るのか、修正後に何が消えるべきかを書く。
- Codex実装タスク:
  1. `GB` event parserへ `gb.trace.snapshot_missing` の正規化を追加する。
  2. `GB` metadata mapperで `trace_link` とmetadata idの対応付けを実装する。
  3. `diagnostics_gb.rs` に `GB048` ruleを追加する。
  4. repair hint generatorへ `emit-snapshot-and-trace-window-for-this-event` を追加する。
  5. fixture、unit test、CLI integration test、HTML snapshot testを追加する。

## 11. 完了条件

- Rust workspace が作成され、`cargo test --workspace` が通る。
- `gb` analyze CLIがmetadata JSONとdiagnostic_events.jsonlを読み込める。
- 本書の 48件すべてに rule、fixture、expected output が存在する。
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

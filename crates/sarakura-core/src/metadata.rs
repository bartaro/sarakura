use crate::model::{BuildMetadata, DiagnosticEvent, SourceMapping};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// Read a UTF-8 JSON object into a generic metadata map, retaining unknown
// fields. File/JSON errors propagate; schema and ROM identity are not validated here.
pub fn load_metadata(path: impl AsRef<Path>) -> Result<BuildMetadata> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read metadata: {}", path.display()))?;
    let raw: BTreeMap<String, Value> = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse metadata json: {}", path.display()))?;
    Ok(BuildMetadata { raw })
}

// Try guessed operation ID, guessed function ID, then PC range in that order.
// The first match wins. If only source hints or a PC remain, return a partial
// low-confidence mapping; matching is heuristic, not proof of executed source.
pub fn correlate_source(
    metadata: &BuildMetadata,
    event: &DiagnosticEvent,
) -> Option<SourceMapping> {
    if let Some(op_id) = event.op_id_guess.as_deref() {
        if let Some(mapping) = find_op_mapping(metadata, op_id, event) {
            return Some(mapping);
        }
    }

    if let Some(function_id) = event.function_id_guess.as_deref() {
        if let Some(mapping) = find_function_mapping(metadata, function_id, event, 0.82) {
            return Some(mapping);
        }
    }

    if let Some(pc) = event.pc.as_deref() {
        if let Some(mapping) = find_pc_range_mapping(metadata, pc, event) {
            return Some(mapping);
        }
    }

    if event.function_hint.is_some() || event.symbol_hint.is_some() || event.pc.is_some() {
        return Some(SourceMapping {
            source_file: None,
            source_line: None,
            function_id: event.function_id_guess.clone(),
            function_name: event.function_hint.clone(),
            symbol_name: event.symbol_hint.clone(),
            op_id: event.op_id_guess.clone(),
            pc: event.pc.clone(),
            bank: event.bank,
            prg_bank: event.prg_bank,
            mapping_confidence: 0.45,
        });
    }

    None
}

// Search the listed operation arrays and their items in order, accepting the
// first recognized ID match. Duplicate IDs are not disambiguated by bank or PC;
// non-array metadata fields are skipped, even when their names appear below.
fn find_op_mapping(
    metadata: &BuildMetadata,
    op_id: &str,
    event: &DiagnosticEvent,
) -> Option<SourceMapping> {
    let arrays = [
        "vram_operations",
        "oam_operations",
        "dma_operations",
        "hdma_operations",
        "vblank_queue_ops",
        "sprite_objects",
        "metasprite_instances",
        "display_configs",
        "scroll_operations",
        "window_operations",
        "stat_configs",
        "timer_configs",
        "input_poll_functions",
        "audio_update_functions",
        "audio_register_writes",
        "ch3_wave_ops",
        "sram_operations",
        "mbc_operations",
        "asset_map",
        "tile_sets",
        "tilemap_operations",
        "palette_operations",
        "audio_assets",
        "ppu_operations",
        "palette_operations",
        "scroll_update_ops",
        "ppu_ctrl_ops",
        "ppu_mask_ops",
        "nametable_operations",
        "attribute_operations",
        "vram_queue_ops",
        "oam_dma_ops",
        "metasprite_draws",
        "sprite_zero_ops",
        "pad_read_functions",
        "apu_register_writes",
        "dmc_usage",
        "mapper_segments",
        "mapper_ops",
        "mapper_irq_ops",
        "chr_bank_ops",
        "farcall_tables",
        "fds_disk_metadata",
        "fds_disk_io_ops",
        "fds_wave_ops",
        "fds_irq_ops",
        "vrc6_audio_ops",
        "vrc7_audio_ops",
        "vrc7_patch_ops",
        "zero_page_layout",
        "ram_layout",
        "stack_config",
        "abi_contracts",
        "overlay_tables",
        "static_cost_estimates",
    ];
    for array_name in arrays {
        if let Some(items) = metadata.raw.get(array_name).and_then(Value::as_array) {
            for item in items {
                if object_has_id(item, op_id) {
                    return Some(mapping_from_object(
                        item,
                        event,
                        Some(op_id.to_string()),
                        0.96,
                    ));
                }
            }
        }
    }
    None
}

// Return the first functions-array item whose recognized ID equals the guess,
// using the caller-supplied confidence without verifying its runtime address.
fn find_function_mapping(
    metadata: &BuildMetadata,
    function_id: &str,
    event: &DiagnosticEvent,
    confidence: f32,
) -> Option<SourceMapping> {
    let functions = metadata.raw.get("functions")?.as_array()?;
    for item in functions {
        if object_has_id(item, function_id) {
            return Some(mapping_from_object(item, event, None, confidence));
        }
    }
    None
}

// Match the first inclusive PC interval after parsing numeric aliases. Bank
// identity is not part of this search. A range missing either valid endpoint
// returns None immediately instead of continuing to later entries.
fn find_pc_range_mapping(
    metadata: &BuildMetadata,
    pc: &str,
    event: &DiagnosticEvent,
) -> Option<SourceMapping> {
    let pc_value = parse_hex_or_dec(pc)?;
    let ranges = metadata.raw.get("pc_ranges")?.as_array()?;
    for item in ranges {
        let start = get_any_u64(item, &["start", "pc_start", "start_pc", "addr_start"])?;
        let end = get_any_u64(item, &["end", "pc_end", "end_pc", "addr_end"])?;
        if pc_value >= start && pc_value <= end {
            return Some(mapping_from_object(
                item,
                event,
                event.op_id_guess.clone(),
                0.72,
            ));
        }
    }
    None
}

// Accept any recognized identifier field with an exact textual/numeric match.
// Fields are alternatives; this does not require a unique or type-specific ID.
fn object_has_id(v: &Value, wanted: &str) -> bool {
    let keys = [
        "id",
        "op_id",
        "operation_id",
        "function_id",
        "symbol_id",
        "queue_id",
        "asset_id",
        "handler_id",
        "callsite_id",
        "intrinsic_id",
        "vram_op_id",
        "oam_op_id",
        "dma_op_id",
        "ppu_op_id",
        "mapper_op_id",
        "fds_op_id",
        "audio_op_id",
    ];
    keys.iter().any(|k| value_matches_id(v.get(*k), wanted))
}

// Compare strings verbatim or JSON numbers in their displayed decimal form.
// Do not coerce booleans, arrays or differently formatted numeric strings.
fn value_matches_id(value: Option<&Value>, wanted: &str) -> bool {
    match value {
        Some(Value::String(s)) => s == wanted,
        Some(Value::Number(n)) => n.to_string() == wanted,
        _ => false,
    }
}

// Combine recognized metadata aliases with event hints. The event PC and bank
// values take precedence; source/function labels generally prefer metadata.
// The supplied confidence is recorded without further validation or clamping.
fn mapping_from_object(
    v: &Value,
    event: &DiagnosticEvent,
    op_id: Option<String>,
    confidence: f32,
) -> SourceMapping {
    SourceMapping {
        source_file: get_any_str(v, &["source_file", "file", "primary_file"]).map(str::to_string),
        source_line: get_any_u64(v, &["source_line", "line", "primary_line"]),
        function_id: get_any_str(v, &["function_id", "id"])
            .map(str::to_string)
            .or_else(|| event.function_id_guess.clone()),
        function_name: get_any_str(v, &["function_name", "name"])
            .map(str::to_string)
            .or_else(|| event.function_hint.clone()),
        symbol_name: get_any_str(v, &["symbol_name", "symbol"])
            .map(str::to_string)
            .or_else(|| event.symbol_hint.clone()),
        op_id: op_id
            .or_else(|| get_any_str(v, &["op_id", "id"]).map(str::to_string))
            .or_else(|| event.op_id_guess.clone()),
        pc: event
            .pc
            .clone()
            .or_else(|| get_any_str(v, &["pc", "start", "pc_start"]).map(str::to_string))
            .or_else(|| get_any_u64(v, &["pc", "start", "pc_start"]).map(format_hex)),
        bank: event.bank.or_else(|| get_any_i64(v, &["bank", "rom_bank"])),
        prg_bank: event.prg_bank.or_else(|| get_any_i64(v, &["prg_bank"])),
        mapping_confidence: confidence,
    }
}

// Return the first alias whose value is a string, skipping absent/wrong-type
// fields. An empty string is a present value and stops the search.
fn get_any_str<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| v.get(*k).and_then(Value::as_str))
}

// Read the first signed JSON integer or parseable unsigned decimal/hex string.
// String results use an as-i64 cast, so values above i64::MAX wrap into negatives;
// negative strings are rejected by the shared unsigned parser.
fn get_any_i64(v: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|k| {
        v.get(*k).and_then(|x| {
            x.as_i64()
                .or_else(|| x.as_str().and_then(parse_hex_or_dec).map(|n| n as i64))
        })
    })
}

// Read the first unsigned JSON integer or decimal/0x-prefixed string among
// the aliases, skipping missing or unparsable values.
fn get_any_u64(v: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|k| {
        v.get(*k)
            .and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(parse_hex_or_dec)))
    })
}

// Trim surrounding whitespace and parse an unsigned value in decimal or with
// a 0x/0X hexadecimal prefix. Invalid text and overflow return None.
fn parse_hex_or_dec(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

// Format uppercase hexadecimal with a lowercase 0x prefix and at least four
// digits; larger addresses are not truncated to 16 bits.
fn format_hex(n: u64) -> String {
    format!("0x{n:04X}")
}

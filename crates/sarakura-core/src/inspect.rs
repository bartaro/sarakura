use crate::model::{
    AiDiagnosticsDocument, BuildMetadata, DiagnosticEvent, EventInspectionSummary, FrameRange,
    MetadataInspectionSummary, RetestDiagnostic, RetestPlan,
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
// Configure advisory input checks. Defaults disable filesystem-reference checks
// and leave the schema substring empty; callers should supply the expected
// producer prefix when they need a meaningful schema-name check.
pub struct EmitterCompatibilityOptions {
    pub expected_schema_prefix: String,
    pub check_snapshot_refs: bool,
    pub check_trace_window_refs: bool,
    pub base_dir: Option<PathBuf>,
}

// Count input records separately from their effective occurrences. Type and
// severity totals use occurrence counts; IDs and evidence references count
// records. Frame bounds use first/last-seen values with frame as the fallback.
pub fn inspect_events(events: &[DiagnosticEvent]) -> EventInspectionSummary {
    let mut summary = EventInspectionSummary {
        events_loaded: events.len(),
        ..EventInspectionSummary::default()
    };

    for event in events {
        let count = event.normalized_count();
        summary.effective_count += count;
        *summary
            .event_types
            .entry(event.event_type.clone())
            .or_insert(0) += count;
        *summary
            .severities
            .entry(normalize_severity(
                event.severity.as_deref().unwrap_or("warn"),
            ))
            .or_insert(0) += count;
        if event.event_id.is_some() {
            summary.event_ids_present += 1;
        }
        if event.snapshot_ref.is_some() {
            summary.snapshot_refs += 1;
        }
        if event.trace_window_ref.is_some() {
            summary.trace_window_refs += 1;
        }
        update_frame_range(&mut summary.frames, event.first_seen.or(event.frame));
        update_frame_range(&mut summary.frames, event.last_seen.or(event.frame));
    }

    summary
}

// Summarize top-level metadata identity and array lengths without validating
// array elements, nested objects or the relationship between metadata and ROM.
pub fn inspect_metadata(metadata: &BuildMetadata) -> MetadataInspectionSummary {
    let mut array_counts = BTreeMap::new();
    for (key, value) in &metadata.raw {
        if let Some(items) = value.as_array() {
            array_counts.insert(key.clone(), items.len());
        }
    }

    MetadataInspectionSummary {
        schema: metadata.schema().map(str::to_string),
        build_id: metadata.build_id().map(str::to_string),
        target: metadata.target().map(str::to_string),
        rom_path: metadata.rom_path().map(str::to_string),
        rom_hash: metadata.rom_hash().map(str::to_string),
        array_counts,
    }
}

// Combine diagnostic retest requirements: take the largest frame budget and
// sorted unique output/expectation names, while preserving per-diagnostic order.
// This creates a plan only; it does not execute or judge the requested rerun.
pub fn build_retest_plan(doc: &AiDiagnosticsDocument) -> RetestPlan {
    let mut expect_absent = BTreeSet::new();
    let mut expect_not_worse = BTreeSet::new();
    let mut required_outputs = BTreeSet::new();
    let mut frames = doc.run.frames_requested;
    let mut diagnostics = Vec::new();

    for diag in &doc.diagnostics {
        frames = frames.max(diag.retest_condition.frames);
        for item in &diag.retest_condition.expect_absent {
            expect_absent.insert(item.clone());
        }
        for item in &diag.retest_condition.expect_not_worse {
            expect_not_worse.insert(item.clone());
        }
        for item in &diag.retest_condition.required_outputs {
            required_outputs.insert(item.clone());
        }
        diagnostics.push(RetestDiagnostic {
            diagnostic_id: diag.diagnostic_id.clone(),
            diagnostic_type: diag.diagnostic_type.clone(),
            severity: diag.severity.clone(),
            catalog_id: diag.catalog_id.clone(),
            repair_target: diag.repair_target.clone(),
            frames: diag.retest_condition.frames,
            expect_absent: diag.retest_condition.expect_absent.clone(),
        });
    }

    RetestPlan {
        schema: "sarakura-retest-plan".to_string(),
        schema_version: 1,
        platform: doc.platform.clone(),
        build_id: doc.build_id.clone(),
        frames,
        expect_absent: expect_absent.into_iter().collect(),
        expect_not_worse: expect_not_worse.into_iter().collect(),
        required_outputs: required_outputs.into_iter().collect(),
        diagnostics,
    }
}

// Create the parent directory and write a pretty JSON retest plan directly to
// the destination. Existing content is replaced; I/O and serialization failures
// are returned with context, without an atomic replacement transaction.
pub fn write_retest_plan(path: impl AsRef<Path>, doc: &AiDiagnosticsDocument) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output dir: {}", parent.display()))?;
    }
    let plan = build_retest_plan(doc);
    let text = serde_json::to_string_pretty(&plan).context("failed to serialize retest plan")?;
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

// Use the metadata-present compatibility path with reference existence checks
// disabled; this convenience wrapper returns warnings instead of rejecting input.
pub fn validate_emitter_compatibility(
    metadata: &BuildMetadata,
    events: &[DiagnosticEvent],
    expected_schema_prefix: &str,
) -> Vec<String> {
    validate_emitter_compatibility_with_options(
        Some(metadata),
        events,
        &EmitterCompatibilityOptions {
            expected_schema_prefix: expected_schema_prefix.to_string(),
            ..EmitterCompatibilityOptions::default()
        },
    )
}

// Inspect metadata identity and at most the first 20 event records for missing
// fields, unusual severity and optional missing references. Schema matching is
// a substring check, not schema validation. Later events are not examined here.
pub fn validate_emitter_compatibility_with_options(
    metadata: Option<&BuildMetadata>,
    events: &[DiagnosticEvent],
    options: &EmitterCompatibilityOptions,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(metadata) = metadata {
        let schema = metadata.schema().unwrap_or("unknown");
        if !schema.contains(&options.expected_schema_prefix) {
            warnings.push(format!(
                "metadata schema {:?} does not look like {} metadata",
                schema, options.expected_schema_prefix
            ));
        }
        if metadata.build_id().is_none() {
            warnings.push("metadata has no build_id".to_string());
        }
    } else {
        warnings.push("metadata was not provided; source correlation will be weaker".to_string());
    }
    if events.is_empty() {
        warnings.push("diagnostic_events input is empty".to_string());
    }
    for event in events.iter().take(20) {
        if event
            .schema
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            warnings.push(format!("event {:?} has no schema", event.event_type));
        }
        if event.schema_version.is_none() {
            warnings.push(format!(
                "event {:?} has no schema_version",
                event.event_type
            ));
        }
        if event.event_type.trim().is_empty() {
            warnings.push("event_type is empty in one of the first 20 events".to_string());
        }
        let severity = normalize_severity(event.severity.as_deref().unwrap_or("warn"));
        if !matches!(severity.as_str(), "error" | "warn" | "info") {
            warnings.push(format!(
                "event {:?} has non-standard severity {:?}",
                event.event_type, event.severity
            ));
        }
        if event.event_id.is_none() {
            warnings.push(format!(
                "event {:?} has no event_id; SARAKURA can still analyze it, but repro traces are weaker",
                event.event_type
            ));
        }
        if event.frame.is_none() && event.first_seen.is_none() && event.last_seen.is_none() {
            warnings.push(format!(
                "event {:?} has no frame/first_seen/last_seen timing anchor",
                event.event_type
            ));
        }
        if event.snapshot_ref.is_none() {
            warnings.push(format!(
                "event {:?} has no snapshot_ref; diagnostic capture is less reproducible",
                event.event_type
            ));
        }
        if event.trace_window_ref.is_none() {
            warnings.push(format!(
                "event {:?} has no trace_window_ref; source diagnosis may need a rerun",
                event.event_type
            ));
        }
        if options.check_snapshot_refs {
            check_ref_exists(
                &mut warnings,
                "snapshot_ref",
                event.snapshot_ref.as_deref(),
                options,
            );
        }
        if options.check_trace_window_refs {
            check_ref_exists(
                &mut warnings,
                "trace_window_ref",
                event.trace_window_ref.as_deref(),
                options,
            );
        }
    }
    warnings
}

// Check only a present reference, relative to base_dir when supplied. Existence
// alone is tested: directories can pass, and file format/content is not read.
// Absent references are handled by the caller's missing-field warnings.
fn check_ref_exists(
    warnings: &mut Vec<String>,
    field: &str,
    value: Option<&str>,
    options: &EmitterCompatibilityOptions,
) {
    let Some(value) = value else {
        return;
    };
    let path = options
        .base_dir
        .as_ref()
        .map(|base| base.join(value))
        .unwrap_or_else(|| PathBuf::from(value));
    if !path.exists() {
        warnings.push(format!("{} {:?} does not exist", field, value));
    }
}

// Widen the minimum/maximum observed frame interval; absent timing adds nothing.
fn update_frame_range(range: &mut FrameRange, frame: Option<u64>) {
    let Some(frame) = frame else {
        return;
    };
    range.first = Some(range.first.map(|x| x.min(frame)).unwrap_or(frame));
    range.last = Some(range.last.map(|x| x.max(frame)).unwrap_or(frame));
}

// Map supported aliases to error, warn or info, preserving other lowercased
// values so compatibility checks can report nonstandard severity names.
fn normalize_severity(severity: &str) -> String {
    match severity.to_ascii_lowercase().as_str() {
        "error" | "err" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "info" | "note" => "info".to_string(),
        other => other.to_string(),
    }
}

// Return the number of object members, or zero for absent and non-object values.
pub fn json_object_len(value: Option<&Value>) -> usize {
    value
        .and_then(Value::as_object)
        .map(|x| x.len())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DiagnosticEvent;

    #[test]
    // Distinguish one input record from three occurrences and verify explicit
    // first/last frame bounds and one snapshot reference.
    fn event_inspection_counts_effective_events() {
        let event = DiagnosticEvent {
            schema: None,
            schema_version: None,
            event_id: Some("evt_1".to_string()),
            event_type: "VRAM_WRITE_OUTSIDE_SAFE_PERIOD".to_string(),
            severity: Some("error".to_string()),
            frame: Some(7),
            scanline: None,
            dot: None,
            cpu_cycle: None,
            pc: None,
            bank: None,
            prg_bank: None,
            addr: None,
            value: None,
            function_id_guess: None,
            symbol_hint: None,
            function_hint: None,
            op_id_guess: None,
            summary_key: None,
            count: Some(3),
            first_seen: Some(5),
            last_seen: Some(9),
            snapshot_ref: Some("snap.json".to_string()),
            trace_window_ref: None,
            extra: BTreeMap::new(),
        };
        let summary = inspect_events(&[event]);
        assert_eq!(summary.events_loaded, 1);
        assert_eq!(summary.effective_count, 3);
        assert_eq!(summary.frames.first, Some(5));
        assert_eq!(summary.frames.last, Some(9));
        assert_eq!(summary.snapshot_refs, 1);
    }
}

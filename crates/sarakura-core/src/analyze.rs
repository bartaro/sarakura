use crate::catalog::catalog_by_event_type;
use crate::confidence::score_confidence;
use crate::metadata::correlate_source;
use crate::model::*;
use crate::packs::diagnostic_matches_pack;
use crate::redaction::redact_document;
use std::collections::BTreeMap;

// Own the metadata, already-loaded events, catalog and analysis policy for one
// analysis. No emulator or filesystem work is carried out by the core pipeline.
pub struct AnalyzeInput {
    pub metadata: BuildMetadata,
    pub events: Vec<DiagnosticEvent>,
    pub catalog: Vec<DiagnosticRule>,
    pub options: AnalyzeOptions,
}

// Aggregate events, correlate source hints and attach catalog-derived repair
// candidates. Apply filters, rank and limit diagnostics before assigning IDs and
// summary counts. Redact selected project labels by default. This analyzes supplied
// observations; it does not validate ROM identity or execute catalog prose.
pub fn analyze(input: AnalyzeInput) -> AiDiagnosticsDocument {
    let catalog_rules_total = input.catalog.len();
    let catalog = catalog_by_event_type(&input.catalog);
    let events_loaded = input.events.len();
    let events = aggregate_events(input.events);
    let events_aggregated = events.len();

    let mut diagnostics = Vec::new();
    for event in &events {
        let rule = catalog.get(&event.event_type);
        let source_mapping = correlate_source(&input.metadata, event);
        let confidence = score_confidence(rule, event, source_mapping.as_ref());
        // An event severity, even an empty/unknown string, takes precedence over the
        // catalog. Only absence falls back to the catalog; normalization defaults to warn.
        let severity = normalized_severity(
            event
                .severity
                .as_deref()
                .or_else(|| rule.map(|r| r.severity.as_str()))
                .unwrap_or("warn"),
        );
        let candidates = build_target_candidates(rule, event, source_mapping.as_ref());
        let repair_target = build_repair_target(rule, event, source_mapping.as_ref(), &candidates);
        let evidence_chain = build_evidence_chain(rule, event, source_mapping.as_ref());
        let safe_patch_hint = rule.map(|r| r.safe_patch_hint.clone()).unwrap_or_else(|| {
            "No SARAKURA catalog rule matches this event_type yet. Add a catalog alias or a dedicated diagnostic rule if the event is intentional."
                .to_string()
        });

        diagnostics.push(AiDiagnostic {
            diagnostic_id: String::new(),
            catalog_id: rule.map(|r| r.id.clone()),
            diagnostic_type: event.event_type.clone(),
            severity,
            phase: rule.map(|r| r.phase.clone()),
            category: rule.map(|r| r.category.clone()),
            confidence,
            source_mapping,
            evidence_chain,
            repair_target,
            target_candidates: candidates,
            safe_patch_hint,
            retest_condition: RetestCondition {
                frames: input.options.frames,
                expect_absent: vec![event.event_type.clone()],
                expect_not_worse: vec!["diagnostics_total".to_string(), event.event_type.clone()],
                required_outputs: vec![
                    "ai_diagnostics.json".to_string(),
                    "diagnostic_summary.json".to_string(),
                    "repair_prompt.md".to_string(),
                    "report.html".to_string(),
                    "retest_plan.json".to_string(),
                ],
            },
            event_count: event.normalized_count(),
            first_seen: event.first_seen.or(event.frame),
            last_seen: event.last_seen.or(event.frame),
            snapshot_ref: event.snapshot_ref.clone(),
            trace_window_ref: event.trace_window_ref.clone(),
            sample_event_ids: event.event_id.iter().cloned().collect(),
        });
    }

    // Record the full aggregated diagnostic count before filtering. Summary and CI
    // counts later describe only retained, possibly truncated diagnostics.
    let diagnostics_before_filter = diagnostics.len();
    diagnostics.retain(|diag| diagnostic_matches_filters(diag, &input.options));

    diagnostics.sort_by(|a, b| {
        severity_rank(&b.severity)
            .cmp(&severity_rank(&a.severity))
            .then_with(|| b.event_count.cmp(&a.event_count))
            .then_with(|| b.confidence.total_cmp(&a.confidence))
            .then_with(|| a.diagnostic_type.cmp(&b.diagnostic_type))
    });

    // A zero limit means unlimited output. Otherwise keep the highest-ranked items,
    // then assign sequential IDs; IDs are not stable across changed filters/rankings.
    if input.options.summary_limit > 0 && diagnostics.len() > input.options.summary_limit {
        diagnostics.truncate(input.options.summary_limit);
    }

    for (idx, diag) in diagnostics.iter_mut().enumerate() {
        diag.diagnostic_id = format!("diag_{:06}", idx + 1);
    }

    let summary = summarize(input.options.frames, &diagnostics);
    let schema_name = match input.options.platform {
        Platform::Gb => "sarakura-gb-ai-diagnostics",
        Platform::Fc => "sarakura-fc-ai-diagnostics",
    };

    let mut document = AiDiagnosticsDocument {
        schema: schema_name.to_string(),
        schema_version: 1,
        producer: "sarakura".to_string(),
        platform: input.options.platform.as_str().to_string(),
        build_id: input.metadata.build_id().map(str::to_string),
        rom: RomSummary {
            path: input.metadata.rom_path().map(str::to_string),
            target: input.metadata.target().map(str::to_string),
            hash: input.metadata.rom_hash().map(str::to_string),
        },
        run: RunSummary {
            frames_requested: input.options.frames,
            diagnostic_summary_limit: input.options.summary_limit,
            events_loaded,
            events_aggregated,
            diagnostics_before_filter,
            catalog_rules_total,
            event_type_filter: input.options.event_type_filter.clone(),
            phase_filter: input.options.phase_filter.clone(),
            pack_filter: input.options.pack_filter.clone(),
            min_severity: input.options.min_severity.clone(),
        },
        summary,
        diagnostics,
    };
    // Apply selected-field redaction after analysis so grouping and correlation use
    // the original labels. This does not sanitize all free-form text in the document.
    if !input.options.allow_project_labels {
        redact_document(&mut document);
    }
    document
}

// Group by stable key, preserving the first record's severity and most payload
// fields. Add counts, fill missing evidence IDs and update last_seen in input
// order; this is not the stronger-severity/min-max merge in normalize_events.
fn aggregate_events(events: Vec<DiagnosticEvent>) -> Vec<DiagnosticEvent> {
    let mut grouped: BTreeMap<String, DiagnosticEvent> = BTreeMap::new();
    for mut event in events {
        let key = event.stable_key();
        if let Some(existing) = grouped.get_mut(&key) {
            existing.count = Some(existing.normalized_count() + event.normalized_count());
            if existing.event_id.is_none() {
                existing.event_id = event.event_id.take();
            }
            if existing.snapshot_ref.is_none() {
                existing.snapshot_ref = event.snapshot_ref.take();
            }
            if existing.trace_window_ref.is_none() {
                existing.trace_window_ref = event.trace_window_ref.take();
            }
            // Retain the first available start rather than the minimum frame. The last
            // record with timing supplies last_seen even if records are out of frame order.
            if existing.first_seen.is_none() {
                existing.first_seen = event.first_seen.or(event.frame);
            }
            existing.last_seen = event.last_seen.or(event.frame).or(existing.last_seen);
        } else {
            if event.count.is_none() {
                event.count = Some(1);
            }
            if event.first_seen.is_none() {
                event.first_seen = event.frame;
            }
            if event.last_seen.is_none() {
                event.last_seen = event.frame;
            }
            grouped.insert(key, event);
        }
    }
    grouped.into_values().collect()
}

// Require every configured filter family to match. Within type/phase/pack
// filters, any requested value may match; event/catalog IDs are exact, phase
// comparison is ASCII case-insensitive and pack matching has its own normalization.
fn diagnostic_matches_filters(diag: &AiDiagnostic, options: &AnalyzeOptions) -> bool {
    if !options.event_type_filter.is_empty()
        && !options
            .event_type_filter
            .iter()
            .any(|x| x == &diag.diagnostic_type || diag.catalog_id.as_ref() == Some(x))
    {
        return false;
    }
    if !options.phase_filter.is_empty() {
        let Some(phase) = diag.phase.as_deref() else {
            return false;
        };
        if !options
            .phase_filter
            .iter()
            .any(|x| x.eq_ignore_ascii_case(phase))
        {
            return false;
        }
    }
    if !diagnostic_matches_pack(
        diag.phase.as_deref(),
        diag.category.as_deref(),
        &options.pack_filter,
    ) {
        return false;
    }
    if let Some(min) = options.min_severity.as_deref() {
        if severity_rank(&diag.severity) < severity_rank(&normalized_severity(min)) {
            return false;
        }
    }
    true
}

// Build ordered evidence descriptions: runtime event, optional capture refs,
// optional mapping, then catalog row. Referenced captures are not opened, and
// source-mapping confidence remains heuristic evidence rather than execution proof.
fn build_evidence_chain(
    rule: Option<&DiagnosticRule>,
    event: &DiagnosticEvent,
    mapping: Option<&SourceMapping>,
) -> Vec<EvidenceItem> {
    let mut out = Vec::new();
    out.push(EvidenceItem {
        kind: "runtime_event".to_string(),
        event_id: event.event_id.clone(),
        event_type: Some(event.event_type.clone()),
        pc: event.pc.clone(),
        bank: event.bank,
        prg_bank: event.prg_bank,
        addr: event.addr.clone(),
        op_id: event.op_id_guess.clone(),
        function_id: event.function_id_guess.clone(),
        note: Some(format!(
            "count={}, first_seen={:?}, last_seen={:?}",
            event.normalized_count(),
            event.first_seen.or(event.frame),
            event.last_seen.or(event.frame)
        )),
    });
    if let Some(snapshot) = &event.snapshot_ref {
        out.push(EvidenceItem {
            kind: "snapshot_ref".to_string(),
            event_id: event.event_id.clone(),
            event_type: Some(event.event_type.clone()),
            pc: event.pc.clone(),
            bank: event.bank,
            prg_bank: event.prg_bank,
            addr: event.addr.clone(),
            op_id: event.op_id_guess.clone(),
            function_id: event.function_id_guess.clone(),
            note: Some(snapshot.clone()),
        });
    }
    if let Some(trace) = &event.trace_window_ref {
        out.push(EvidenceItem {
            kind: "trace_window_ref".to_string(),
            event_id: event.event_id.clone(),
            event_type: Some(event.event_type.clone()),
            pc: event.pc.clone(),
            bank: event.bank,
            prg_bank: event.prg_bank,
            addr: event.addr.clone(),
            op_id: event.op_id_guess.clone(),
            function_id: event.function_id_guess.clone(),
            note: Some(trace.clone()),
        });
    }
    if let Some(mapping) = mapping {
        out.push(EvidenceItem {
            kind: "source_mapping".to_string(),
            event_id: None,
            event_type: None,
            pc: mapping.pc.clone(),
            bank: mapping.bank,
            prg_bank: mapping.prg_bank,
            addr: event.addr.clone(),
            op_id: mapping.op_id.clone(),
            function_id: mapping.function_id.clone(),
            note: mapping
                .source_file
                .as_ref()
                .map(|f| match mapping.source_line {
                    Some(line) => format!(
                        "{}:{} confidence={:.2}",
                        f, line, mapping.mapping_confidence
                    ),
                    None => format!("{} confidence={:.2}", f, mapping.mapping_confidence),
                }),
        });
    }
    if let Some(rule) = rule {
        out.push(EvidenceItem {
            kind: "catalog_rule".to_string(),
            event_id: None,
            event_type: Some(rule.event_type.clone()),
            pc: None,
            bank: None,
            prg_bank: None,
            addr: None,
            op_id: None,
            function_id: None,
            note: Some(format!("{} / {} / {}", rule.id, rule.phase, rule.category)),
        });
    }
    out
}

// Use catalog candidate order as one-based rank, attaching unique available
// references. Unknown event types get one unknown_rule candidate; a known rule
// with no candidates yields an empty list. No patch applicability is tested.
fn build_target_candidates(
    rule: Option<&DiagnosticRule>,
    event: &DiagnosticEvent,
    mapping: Option<&SourceMapping>,
) -> Vec<TargetCandidate> {
    let Some(rule) = rule else {
        return vec![TargetCandidate {
            target_type: "unknown_rule".to_string(),
            rank: 1,
            reason: "This event_type is not registered in the catalog.".to_string(),
            metadata_refs: vec![event.event_type.clone()],
        }];
    };

    rule.repair_target_candidates
        .iter()
        .enumerate()
        .map(|(i, candidate)| TargetCandidate {
            target_type: candidate.clone(),
            rank: (i + 1) as u32,
            reason: if i == 0 {
                "Primary candidate from the v3 specification; review first as a candidate for a safe automated repair.".to_string()
            } else {
                "Alternative candidate from the v3 specification; consider it if the primary candidate does not apply.".to_string()
            },
            metadata_refs: unique_strings([
                mapping.and_then(|m| m.op_id.clone()),
                mapping.and_then(|m| m.function_id.clone()),
                event.op_id_guess.clone(),
                event.function_id_guess.clone(),
                event.snapshot_ref.clone(),
                event.trace_window_ref.clone(),
            ]),
        })
        .collect()
}

// Choose the first candidate, then the first catalog target, else manual_inspection.
// Take source labels from mapping with ID/name hints as fallbacks. Detection
// prose becomes the reason; no source edit is made or proven safe here.
fn build_repair_target(
    rule: Option<&DiagnosticRule>,
    event: &DiagnosticEvent,
    mapping: Option<&SourceMapping>,
    candidates: &[TargetCandidate],
) -> RepairTarget {
    let target_type = candidates
        .first()
        .map(|c| c.target_type.clone())
        .or_else(|| rule.and_then(|r| r.repair_target_candidates.first().cloned()))
        .unwrap_or_else(|| "manual_inspection".to_string());

    RepairTarget {
        target_type,
        primary_file: mapping.and_then(|m| m.source_file.clone()),
        primary_line: mapping.and_then(|m| m.source_line),
        function_id: mapping
            .and_then(|m| m.function_id.clone())
            .or_else(|| event.function_id_guess.clone()),
        function_name: mapping
            .and_then(|m| m.function_name.clone())
            .or_else(|| event.function_hint.clone()),
        op_id: mapping
            .and_then(|m| m.op_id.clone())
            .or_else(|| event.op_id_guess.clone()),
        reason: rule
            .map(|r| r.detection_condition.clone())
            .unwrap_or_else(|| {
                "This repair target was generated from a runtime diagnostic event.".to_string()
            }),
    }
}

// Count retained diagnostic records, not their summed event occurrences.
// frames_analyzed is the caller's configured frame value, not a measured span.
// Any present partial mapping counts as mapped even without a source file/line.
fn summarize(frames: u64, diagnostics: &[AiDiagnostic]) -> DiagnosticSummary {
    let mut summary = DiagnosticSummary {
        frames_analyzed: frames,
        diagnostics_total: diagnostics.len(),
        ..DiagnosticSummary::default()
    };
    for diag in diagnostics {
        match diag.severity.as_str() {
            "error" => summary.errors += 1,
            "warn" | "warning" => summary.warnings += 1,
            _ => summary.infos += 1,
        }
        if diag.source_mapping.is_none() {
            summary.unmapped_diagnostics += 1;
        }
        *summary
            .event_types
            .entry(diag.diagnostic_type.clone())
            .or_insert(0) += 1;
        *summary
            .phases
            .entry(diag.phase.clone().unwrap_or_else(|| "unknown".to_string()))
            .or_insert(0) += 1;
        *summary
            .categories
            .entry(
                diag.category
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            )
            .or_insert(0) += 1;
        *summary
            .repair_targets
            .entry(diag.repair_target.target_type.clone())
            .or_insert(0) += 1;
    }
    summary
}

// Normalize error/warn/info aliases and map unrecognized values to warn.
// Whitespace is not stripped before matching.
fn normalized_severity(severity: &str) -> String {
    match severity.to_ascii_lowercase().as_str() {
        "error" | "err" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "info" | "note" => "info".to_string(),
        _ => "warn".to_string(),
    }
}

// Rank error above warn above info. Unlike normalized_severity, this helper
// gives unknown inputs the informational rank; callers normalize when required.
pub fn severity_rank(severity: &str) -> u8 {
    match severity {
        "error" => 3,
        "warn" | "warning" => 2,
        "info" => 1,
        _ => match severity.to_ascii_lowercase().as_str() {
            "error" | "err" => 3,
            "warn" | "warning" => 2,
            _ => 1,
        },
    }
}

// Drop absent references and exact duplicates while preserving first occurrence
// order. Present empty strings remain valid entries.
fn unique_strings<const N: usize>(items: [Option<String>; N]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items.into_iter().flatten() {
        if !out.iter().any(|x| x == &item) {
            out.push(item);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    // Create a synthetic mapped event with evidence references for pipeline tests.
    fn sample_event(event_type: &str, severity: &str) -> DiagnosticEvent {
        DiagnosticEvent {
            schema: None,
            schema_version: None,
            event_id: Some("evt_1".to_string()),
            event_type: event_type.to_string(),
            severity: Some(severity.to_string()),
            frame: Some(1),
            scanline: None,
            dot: None,
            cpu_cycle: None,
            pc: Some("0x415a".to_string()),
            bank: Some(1),
            prg_bank: None,
            addr: Some("0x9800".to_string()),
            value: None,
            function_id_guess: Some("func_draw".to_string()),
            symbol_hint: None,
            function_hint: None,
            op_id_guess: None,
            summary_key: None,
            count: None,
            first_seen: None,
            last_seen: None,
            snapshot_ref: Some("snap_1.json".to_string()),
            trace_window_ref: Some("trace_1.bin".to_string()),
            extra: BTreeMap::new(),
        }
    }

    // Provide a minimal function/source mapping and synthetic ROM identity;
    // these fixture labels are not a real build or ROM fingerprint.
    fn sample_metadata() -> BuildMetadata {
        let mut raw = BTreeMap::new();
        raw.insert("schema".to_string(), json!("kitaqgb-ai-build-metadata"));
        raw.insert("build_id".to_string(), json!("b1"));
        raw.insert("target".to_string(), json!("gbc"));
        raw.insert(
            "rom".to_string(),
            json!({"path":"game.gb", "hash":"sha256-demo"}),
        );
        raw.insert(
            "functions".to_string(),
            json!([{
                "function_id":"func_draw",
                "function_name":"draw",
                "source_file":"main.c",
                "source_line":42
            }]),
        );
        BuildMetadata { raw }
    }

    // Provide one error rule with a VBlank-queue repair candidate for tests.
    fn sample_catalog() -> Vec<DiagnosticRule> {
        vec![DiagnosticRule {
            id: "GBD001".to_string(),
            event_type: "VRAM_WRITE_OUTSIDE_SAFE_PERIOD".to_string(),
            phase: "MVP-1".to_string(),
            severity: "error".to_string(),
            category: "PPU/VRAM".to_string(),
            detection_condition: "unsafe vram write".to_string(),
            required_metadata: vec![],
            event_fields: vec![],
            repair_target_candidates: vec!["vblank_queue".to_string()],
            safe_patch_hint: "queue it".to_string(),
            fixtures: FixturePair::default(),
            codex_tasks: vec![],
        }]
    }

    #[test]
    // Verify source correlation, retained project labels and summary counts when
    // allow_project_labels is enabled.
    fn produces_ai_diagnostic_from_event() {
        let doc = analyze(AnalyzeInput {
            metadata: sample_metadata(),
            events: vec![sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "error")],
            catalog: sample_catalog(),
            options: AnalyzeOptions {
                platform: Platform::Gb,
                frames: 10,
                summary_limit: 10,
                allow_project_labels: true,
                ..AnalyzeOptions::default()
            },
        });
        assert_eq!(doc.diagnostics.len(), 1);
        assert_eq!(
            doc.diagnostics[0].repair_target.primary_file.as_deref(),
            Some("main.c")
        );
        assert_eq!(doc.rom.hash.as_deref(), Some("sha256-demo"));
        assert_eq!(doc.run.events_loaded, 1);
        assert_eq!(doc.summary.repair_targets.get("vblank_queue"), Some(&1));
    }

    #[test]
    // Verify default replacement of selected project/source labels and removal
    // of the ROM hash; this does not test every free-form field for anonymization.
    fn redacts_project_labels_by_default() {
        let doc = analyze(AnalyzeInput {
            metadata: sample_metadata(),
            events: vec![sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "error")],
            catalog: sample_catalog(),
            options: AnalyzeOptions {
                platform: Platform::Gb,
                frames: 10,
                summary_limit: 10,
                ..AnalyzeOptions::default()
            },
        });
        assert_eq!(doc.build_id.as_deref(), Some("project_0001"));
        assert_eq!(doc.rom.path.as_deref(), Some("input_0001"));
        assert_eq!(doc.rom.hash, None);
        assert_eq!(
            doc.diagnostics[0].repair_target.primary_file.as_deref(),
            Some("source_0001")
        );
        assert_eq!(
            doc.diagnostics[0]
                .source_mapping
                .as_ref()
                .and_then(|mapping| mapping.function_name.as_deref()),
            Some("function_0001")
        );
    }

    #[test]
    // Check that two records with the same explicit summary key become one
    // diagnostic with five occurrences.
    fn aggregates_duplicate_events() {
        let mut first = sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "error");
        first.summary_key = Some("same".to_string());
        first.count = Some(2);
        let mut second = sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "error");
        second.summary_key = Some("same".to_string());
        second.count = Some(3);
        second.frame = Some(8);
        let doc = analyze(AnalyzeInput {
            metadata: sample_metadata(),
            events: vec![first, second],
            catalog: sample_catalog(),
            options: AnalyzeOptions {
                platform: Platform::Gb,
                frames: 10,
                summary_limit: 10,
                ..AnalyzeOptions::default()
            },
        });
        assert_eq!(doc.run.events_loaded, 2);
        assert_eq!(doc.run.events_aggregated, 1);
        assert_eq!(doc.diagnostics[0].event_count, 5);
    }

    #[test]
    // Confirm an explicit event warning overrides the catalog's error default.
    fn event_severity_overrides_catalog_default() {
        let doc = analyze(AnalyzeInput {
            metadata: sample_metadata(),
            events: vec![sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "warn")],
            catalog: sample_catalog(),
            options: AnalyzeOptions {
                platform: Platform::Gb,
                frames: 10,
                summary_limit: 10,
                ..AnalyzeOptions::default()
            },
        });
        assert_eq!(doc.diagnostics[0].severity, "warn");
        assert_eq!(doc.summary.warnings, 1);
        assert_eq!(doc.summary.errors, 0);
    }

    #[test]
    // Reject a diagnostic whose phase differs and preserve its prefilter count.
    // The severity threshold is configured but is not independently challenged here.
    fn filters_by_phase_and_min_severity() {
        let doc = analyze(AnalyzeInput {
            metadata: sample_metadata(),
            events: vec![sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "error")],
            catalog: sample_catalog(),
            options: AnalyzeOptions {
                platform: Platform::Gb,
                frames: 10,
                summary_limit: 10,
                phase_filter: vec!["MVP-2".to_string()],
                pack_filter: Vec::new(),
                min_severity: Some("error".to_string()),
                event_type_filter: Vec::new(),
                allow_project_labels: false,
            },
        });
        assert!(doc.diagnostics.is_empty());
        assert_eq!(doc.run.diagnostics_before_filter, 1);
    }

    #[test]
    // Check inclusion by the matching PPU domain and exclusion by the audio domain.
    fn filters_by_diagnostic_pack() {
        let doc = analyze(AnalyzeInput {
            metadata: sample_metadata(),
            events: vec![sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "error")],
            catalog: sample_catalog(),
            options: AnalyzeOptions {
                platform: Platform::Gb,
                frames: 10,
                summary_limit: 10,
                pack_filter: vec!["ppu".to_string()],
                ..AnalyzeOptions::default()
            },
        });
        assert_eq!(doc.diagnostics.len(), 1);
        assert_eq!(doc.run.pack_filter, vec!["ppu".to_string()]);

        let none = analyze(AnalyzeInput {
            metadata: sample_metadata(),
            events: vec![sample_event("VRAM_WRITE_OUTSIDE_SAFE_PERIOD", "error")],
            catalog: sample_catalog(),
            options: AnalyzeOptions {
                platform: Platform::Gb,
                frames: 10,
                summary_limit: 10,
                pack_filter: vec!["audio".to_string()],
                ..AnalyzeOptions::default()
            },
        });
        assert!(none.diagnostics.is_empty());
    }
}

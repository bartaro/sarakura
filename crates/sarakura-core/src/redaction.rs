use crate::model::AiDiagnosticsDocument;

/// Replaces project-private labels and paths with deterministic generic labels.
/// Runtime addresses, rule IDs, counts, confidence, and repair categories remain
/// available so the result stays useful for automated diagnosis.
pub fn redact_document(document: &mut AiDiagnosticsDocument) {
    if document.build_id.is_some() {
        document.build_id = Some("project_0001".to_string());
    }
    if document.rom.path.is_some() {
        document.rom.path = Some("input_0001".to_string());
    }
    document.rom.hash = None;

    for diagnostic in &mut document.diagnostics {
        if let Some(mapping) = &mut diagnostic.source_mapping {
            replace_if_present(&mut mapping.source_file, "source_0001");
            replace_if_present(&mut mapping.function_id, "function_0001");
            replace_if_present(&mut mapping.function_name, "function_0001");
            replace_if_present(&mut mapping.symbol_name, "symbol_0001");
            replace_if_present(&mut mapping.op_id, "operation_0001");
        }

        replace_if_present(&mut diagnostic.repair_target.primary_file, "source_0001");
        replace_if_present(&mut diagnostic.repair_target.function_id, "function_0001");
        replace_if_present(&mut diagnostic.repair_target.function_name, "function_0001");
        replace_if_present(&mut diagnostic.repair_target.op_id, "operation_0001");

        for candidate in &mut diagnostic.target_candidates {
            for (index, metadata_ref) in candidate.metadata_refs.iter_mut().enumerate() {
                *metadata_ref = format!("metadata_ref_{:04}", index + 1);
            }
        }

        for evidence in &mut diagnostic.evidence_chain {
            replace_if_present(&mut evidence.op_id, "operation_0001");
            replace_if_present(&mut evidence.function_id, "function_0001");
            match evidence.kind.as_str() {
                "snapshot_ref" => evidence.note = Some("snapshot_0001".to_string()),
                "trace_window_ref" => evidence.note = Some("trace_0001".to_string()),
                "source_mapping" => evidence.note = Some("source_0001".to_string()),
                _ => {}
            }
        }

        replace_if_present(&mut diagnostic.snapshot_ref, "snapshot_0001");
        replace_if_present(&mut diagnostic.trace_window_ref, "trace_0001");
    }
}

fn replace_if_present(value: &mut Option<String>, replacement: &str) {
    if value.is_some() {
        *value = Some(replacement.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AiDiagnosticsDocument, DiagnosticSummary, RomSummary, RunSummary};

    #[test]
    fn removes_top_level_path_hash_and_build_label() {
        let mut doc = AiDiagnosticsDocument {
            schema: "schema".to_string(),
            schema_version: 1,
            producer: "sarakura".to_string(),
            platform: "fc".to_string(),
            build_id: Some("private-build".to_string()),
            rom: RomSummary {
                path: Some("private/input.nes".to_string()),
                target: Some("nes".to_string()),
                hash: Some("private-hash".to_string()),
            },
            run: RunSummary {
                frames_requested: 1,
                diagnostic_summary_limit: 1,
                events_loaded: 0,
                events_aggregated: 0,
                diagnostics_before_filter: 0,
                catalog_rules_total: 0,
                event_type_filter: Vec::new(),
                phase_filter: Vec::new(),
                pack_filter: Vec::new(),
                min_severity: None,
            },
            summary: DiagnosticSummary::default(),
            diagnostics: Vec::new(),
        };
        redact_document(&mut doc);
        assert_eq!(doc.build_id.as_deref(), Some("project_0001"));
        assert_eq!(doc.rom.path.as_deref(), Some("input_0001"));
        assert_eq!(doc.rom.hash, None);
        assert_eq!(doc.rom.target.as_deref(), Some("nes"));
    }
}

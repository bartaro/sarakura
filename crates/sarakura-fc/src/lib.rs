use anyhow::Result;
use sarakura_core::{
    analyze, load_catalog_from_str, AiDiagnosticsDocument, AnalyzeInput, AnalyzeOptions,
    BuildMetadata, DiagnosticEvent, Platform,
};

const FC_CATALOG_JSON: &str = include_str!("fc_catalog.json");

// Parse the embedded FC rule catalog on each call; return owned rules or
// propagate the parse error without consulting an emulator.
pub fn fc_catalog() -> Result<Vec<sarakura_core::DiagnosticRule>> {
    load_catalog_from_str(FC_CATALOG_JSON)
}

// Analyze supplied FC metadata and events with default filtering and label
// redaction. The frame budget is recorded, not executed here.
pub fn analyze_fc(
    metadata: BuildMetadata,
    events: Vec<DiagnosticEvent>,
    frames: u64,
    summary_limit: usize,
) -> Result<AiDiagnosticsDocument> {
    analyze_fc_with_options(
        metadata,
        events,
        AnalyzeOptions {
            platform: Platform::Fc,
            frames,
            summary_limit,
            ..AnalyzeOptions::default()
        },
    )
}

// Select FC unconditionally, load the FC catalog, and pass caller options
// and input data to the shared analysis pipeline.
pub fn analyze_fc_with_options(
    metadata: BuildMetadata,
    events: Vec<DiagnosticEvent>,
    mut options: AnalyzeOptions,
) -> Result<AiDiagnosticsDocument> {
    options.platform = Platform::Fc;
    let catalog = fc_catalog()?;
    Ok(analyze(AnalyzeInput {
        metadata,
        events,
        catalog,
        options,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sarakura_core::load_events_jsonl;
    use std::path::PathBuf;

    #[test]
    // Check the catalog count, selected legacy IDs and required MMC1/SUROM IDs,
    // including the PPU write-risk mapping. Emitter behavior is not tested here.
    fn catalog_has_surom_rules() {
        let rules = fc_catalog().unwrap();
        assert_eq!(rules.len(), 74);
        assert!(rules.iter().any(|r| r.id == "FCD001"));
        assert!(rules.iter().any(|r| r.id == "FCD063"));
        assert!(rules.iter().any(|r| r.id == "FCD064" && r.event_type == "PPU_REGISTER_WRITE_RISK"));
        for id in [
            "FC_MMC1_OUTER_BANK_MISMATCH",
            "FC_MMC1_COMMON_REPLICA_DIVERGENCE",
            "FC_MMC1_SERIAL_WRITE_INTERRUPTED",
            "FC_MMC1_CONSECUTIVE_WRITE_IGNORED",
            "FC_MMC1_CHR_MODE_UNSAFE",
            "FC_MMC1_PRG_RAM_DISABLED",
            "FC_SUROM_BANK_RANGE",
            "FC_SUROM_HEADER_MISMATCH",
            "FC_SUROM_VECTOR_REPLICA_MISMATCH",
            "FC_SUROM_METADATA_MAPPING_MISMATCH",
        ] {
            assert!(rules.iter().any(|rule| rule.id == id), "missing {id}");
        }
    }

    #[test]
    // Analyze the bundled synthetic fixture and check label redaction plus the
    // two diagnostics' evidence, target and retest fields. No emulator is run.
    fn synthetic_surom_fixture_produces_redacted_actionable_diagnostics() {
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/surom512");
        let metadata: BuildMetadata =
            serde_json::from_str(&std::fs::read_to_string(fixture.join("metadata.json")).unwrap())
                .unwrap();
        let events = load_events_jsonl(fixture.join("events.jsonl")).unwrap();
        let doc = analyze_fc(metadata, events, 4, 20).unwrap();

        assert_eq!(doc.build_id.as_deref(), Some("project_0001"));
        assert_eq!(doc.rom.path.as_deref(), Some("input_0001"));
        assert_eq!(doc.rom.hash, None);
        assert_eq!(doc.diagnostics.len(), 2);
        assert!(doc.diagnostics.iter().all(|diagnostic| {
            diagnostic.catalog_id.is_some()
                && !diagnostic.evidence_chain.is_empty()
                && !diagnostic.repair_target.target_type.is_empty()
                && diagnostic.confidence > 0.0
                && !diagnostic.retest_condition.expect_absent.is_empty()
        }));
    }
}

use anyhow::Result;
use sarakura_core::{
    analyze, load_catalog_from_str, AiDiagnosticsDocument, AnalyzeInput, AnalyzeOptions,
    BuildMetadata, DiagnosticEvent, Platform,
};

const GB_CATALOG_JSON: &str = include_str!("gb_catalog.json");

pub fn gb_catalog() -> Result<Vec<sarakura_core::DiagnosticRule>> {
    load_catalog_from_str(GB_CATALOG_JSON)
}

pub fn analyze_gb(
    metadata: BuildMetadata,
    events: Vec<DiagnosticEvent>,
    frames: u64,
    summary_limit: usize,
) -> Result<AiDiagnosticsDocument> {
    analyze_gb_with_options(
        metadata,
        events,
        AnalyzeOptions {
            platform: Platform::Gb,
            frames,
            summary_limit,
            ..AnalyzeOptions::default()
        },
    )
}

pub fn analyze_gb_with_options(
    metadata: BuildMetadata,
    events: Vec<DiagnosticEvent>,
    mut options: AnalyzeOptions,
) -> Result<AiDiagnosticsDocument> {
    options.platform = Platform::Gb;
    let catalog = gb_catalog()?;
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

    #[test]
    fn catalog_has_50_rules() {
        let rules = gb_catalog().unwrap();
        assert_eq!(rules.len(), 50);
        assert!(rules.iter().any(|r| r.id == "GBD001"));
        assert!(rules.iter().any(|r| r.id == "GBD049"));
        assert!(rules.iter().any(|r| r.id == "GBD050"));
    }
}

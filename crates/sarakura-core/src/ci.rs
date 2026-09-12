use crate::model::AiDiagnosticsDocument;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountItem {
    pub key: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiSummary {
    pub schema: String,
    pub schema_version: u32,
    pub platform: String,
    pub build_id: Option<String>,
    pub fail_on: String,
    pub status: String,
    pub exit_code: i32,
    pub diagnostics_total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub unmapped_diagnostics: usize,
    pub top_event_types: Vec<CountItem>,
    pub top_repair_targets: Vec<CountItem>,
    pub retest_plan_required: bool,
    pub message: String,
}

pub fn build_ci_summary(doc: &AiDiagnosticsDocument, fail_on: &str) -> CiSummary {
    let fail_on = normalize_fail_on(fail_on);
    let should_fail = match fail_on.as_str() {
        "never" => false,
        "error" => doc.summary.errors > 0,
        "warn" => doc.summary.errors + doc.summary.warnings > 0,
        "info" => doc.summary.diagnostics_total > 0,
        _ => doc.summary.errors > 0,
    };
    let status = if should_fail { "failed" } else { "passed" }.to_string();
    let exit_code = if should_fail { 1 } else { 0 };
    let message = if should_fail {
        format!(
            "SARAKURA CI gate failed: fail_on={} total={} errors={} warnings={} infos={}",
            fail_on,
            doc.summary.diagnostics_total,
            doc.summary.errors,
            doc.summary.warnings,
            doc.summary.infos
        )
    } else {
        format!(
            "SARAKURA CI gate passed: fail_on={} total={} errors={} warnings={} infos={}",
            fail_on,
            doc.summary.diagnostics_total,
            doc.summary.errors,
            doc.summary.warnings,
            doc.summary.infos
        )
    };

    CiSummary {
        schema: "sarakura-ci-summary".to_string(),
        schema_version: 1,
        platform: doc.platform.clone(),
        build_id: doc.build_id.clone(),
        fail_on,
        status,
        exit_code,
        diagnostics_total: doc.summary.diagnostics_total,
        errors: doc.summary.errors,
        warnings: doc.summary.warnings,
        infos: doc.summary.infos,
        unmapped_diagnostics: doc.summary.unmapped_diagnostics,
        top_event_types: top_items(&doc.summary.event_types, 10),
        top_repair_targets: top_items(&doc.summary.repair_targets, 10),
        retest_plan_required: doc.summary.diagnostics_total > 0,
        message,
    }
}

fn normalize_fail_on(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "never" => "never".to_string(),
        "error" | "err" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "info" | "all" => "info".to_string(),
        _ => "error".to_string(),
    }
}

fn top_items(map: &BTreeMap<String, usize>, limit: usize) -> Vec<CountItem> {
    let mut items: Vec<_> = map
        .iter()
        .map(|(key, count)| CountItem {
            key: key.clone(),
            count: *count,
        })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    items.truncate(limit);
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagnosticSummary, RomSummary, RunSummary};

    fn sample_doc(errors: usize, warnings: usize, infos: usize) -> AiDiagnosticsDocument {
        AiDiagnosticsDocument {
            schema: "sarakura-gb-ai-diagnostics".to_string(),
            schema_version: 1,
            producer: "sarakura".to_string(),
            platform: "gb".to_string(),
            build_id: Some("b1".to_string()),
            rom: RomSummary {
                path: None,
                target: None,
                hash: None,
            },
            run: RunSummary {
                frames_requested: 300,
                diagnostic_summary_limit: 200,
                events_loaded: 0,
                events_aggregated: 0,
                diagnostics_before_filter: 0,
                catalog_rules_total: 0,
                event_type_filter: vec![],
                phase_filter: vec![],
                pack_filter: vec![],
                min_severity: None,
            },
            summary: DiagnosticSummary {
                diagnostics_total: errors + warnings + infos,
                errors,
                warnings,
                infos,
                ..DiagnosticSummary::default()
            },
            diagnostics: vec![],
        }
    }

    #[test]
    fn ci_summary_fails_on_error() {
        let doc = sample_doc(1, 0, 0);
        let summary = build_ci_summary(&doc, "error");
        assert_eq!(summary.status, "failed");
        assert_eq!(summary.exit_code, 1);
    }

    #[test]
    fn ci_summary_passes_when_policy_allows() {
        let doc = sample_doc(0, 1, 0);
        let summary = build_ci_summary(&doc, "error");
        assert_eq!(summary.status, "passed");
        assert_eq!(summary.exit_code, 0);
    }
}

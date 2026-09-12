use crate::model::{AiDiagnostic, AiDiagnosticsDocument};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineDeltaPolicy {
    pub fail_on_new: String,
    pub fail_on_regression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaselineDeltaSummary {
    pub baseline_total: usize,
    pub current_total: usize,
    pub new_total: usize,
    pub new_errors: usize,
    pub new_warnings: usize,
    pub new_infos: usize,
    pub resolved_total: usize,
    pub persisting_total: usize,
    pub regressed_total: usize,
    pub improved_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineDeltaItem {
    pub key: String,
    pub diagnostic_type: String,
    pub catalog_id: Option<String>,
    pub repair_target: String,
    pub primary_file: Option<String>,
    pub primary_line: Option<u64>,
    pub baseline_diagnostic_id: Option<String>,
    pub current_diagnostic_id: Option<String>,
    pub baseline_severity: Option<String>,
    pub current_severity: Option<String>,
    pub baseline_event_count: Option<u64>,
    pub current_event_count: Option<u64>,
    pub baseline_confidence: Option<f32>,
    pub current_confidence: Option<f32>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineDeltaReport {
    pub schema: String,
    pub schema_version: u32,
    pub platform: String,
    pub baseline_build_id: Option<String>,
    pub current_build_id: Option<String>,
    pub status: String,
    pub exit_code: i32,
    pub policy: BaselineDeltaPolicy,
    pub summary: BaselineDeltaSummary,
    pub new_diagnostics: Vec<BaselineDeltaItem>,
    pub resolved_diagnostics: Vec<BaselineDeltaItem>,
    pub persisting_diagnostics: Vec<BaselineDeltaItem>,
    pub regressed_diagnostics: Vec<BaselineDeltaItem>,
    pub improved_diagnostics: Vec<BaselineDeltaItem>,
    pub message: String,
}

pub fn build_baseline_delta(
    baseline: &AiDiagnosticsDocument,
    current: &AiDiagnosticsDocument,
    fail_on_new: &str,
    fail_on_regression: &str,
) -> BaselineDeltaReport {
    let fail_on_new = normalize_fail_on(fail_on_new);
    let fail_on_regression = normalize_fail_on(fail_on_regression);
    let baseline_map = diagnostic_map(&baseline.diagnostics);
    let current_map = diagnostic_map(&current.diagnostics);
    let keys = baseline_map
        .keys()
        .chain(current_map.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut new_diagnostics = Vec::new();
    let mut resolved_diagnostics = Vec::new();
    let mut persisting_diagnostics = Vec::new();
    let mut regressed_diagnostics = Vec::new();
    let mut improved_diagnostics = Vec::new();

    for key in keys {
        match (baseline_map.get(&key), current_map.get(&key)) {
            (None, Some(current_diag)) => {
                new_diagnostics.push(delta_item(
                    &key,
                    None,
                    Some(current_diag),
                    "new diagnostic type/target combination",
                ));
            }
            (Some(baseline_diag), None) => {
                resolved_diagnostics.push(delta_item(
                    &key,
                    Some(baseline_diag),
                    None,
                    "diagnostic disappeared compared to baseline",
                ));
            }
            (Some(baseline_diag), Some(current_diag)) => {
                let base_rank = severity_rank(&baseline_diag.severity);
                let current_rank = severity_rank(&current_diag.severity);
                if current_rank > base_rank || current_diag.event_count > baseline_diag.event_count
                {
                    regressed_diagnostics.push(delta_item(
                        &key,
                        Some(baseline_diag),
                        Some(current_diag),
                        "diagnostic severity or event_count became worse",
                    ));
                } else if current_rank < base_rank
                    || current_diag.event_count < baseline_diag.event_count
                {
                    improved_diagnostics.push(delta_item(
                        &key,
                        Some(baseline_diag),
                        Some(current_diag),
                        "diagnostic severity or event_count improved",
                    ));
                } else {
                    persisting_diagnostics.push(delta_item(
                        &key,
                        Some(baseline_diag),
                        Some(current_diag),
                        "diagnostic still present with same severity and event_count",
                    ));
                }
            }
            (None, None) => {}
        }
    }

    sort_items(&mut new_diagnostics);
    sort_items(&mut resolved_diagnostics);
    sort_items(&mut persisting_diagnostics);
    sort_items(&mut regressed_diagnostics);
    sort_items(&mut improved_diagnostics);

    let mut summary = BaselineDeltaSummary {
        baseline_total: baseline.summary.diagnostics_total,
        current_total: current.summary.diagnostics_total,
        new_total: new_diagnostics.len(),
        resolved_total: resolved_diagnostics.len(),
        persisting_total: persisting_diagnostics.len(),
        regressed_total: regressed_diagnostics.len(),
        improved_total: improved_diagnostics.len(),
        ..BaselineDeltaSummary::default()
    };
    for item in &new_diagnostics {
        match item
            .current_severity
            .as_deref()
            .map(normalize_severity)
            .as_deref()
        {
            Some("error") => summary.new_errors += 1,
            Some("warn") => summary.new_warnings += 1,
            _ => summary.new_infos += 1,
        }
    }

    let new_gate_failed = new_diagnostics
        .iter()
        .any(|item| severity_meets(item.current_severity.as_deref(), &fail_on_new));
    let regression_gate_failed = regressed_diagnostics
        .iter()
        .any(|item| severity_meets(item.current_severity.as_deref(), &fail_on_regression));
    let failed = new_gate_failed || regression_gate_failed;
    let status = if failed { "failed" } else { "passed" }.to_string();
    let exit_code = if failed { 1 } else { 0 };
    let message = if failed {
        format!(
            "SARAKURA baseline delta failed: new={} regressed={} fail_on_new={} fail_on_regression={}",
            summary.new_total, summary.regressed_total, fail_on_new, fail_on_regression
        )
    } else {
        format!(
            "SARAKURA baseline delta passed: new={} resolved={} regressed={} improved={}",
            summary.new_total,
            summary.resolved_total,
            summary.regressed_total,
            summary.improved_total
        )
    };

    BaselineDeltaReport {
        schema: "sarakura-baseline-delta".to_string(),
        schema_version: 1,
        platform: current.platform.clone(),
        baseline_build_id: baseline.build_id.clone(),
        current_build_id: current.build_id.clone(),
        status,
        exit_code,
        policy: BaselineDeltaPolicy {
            fail_on_new,
            fail_on_regression,
        },
        summary,
        new_diagnostics,
        resolved_diagnostics,
        persisting_diagnostics,
        regressed_diagnostics,
        improved_diagnostics,
        message,
    }
}

pub fn render_baseline_delta_markdown(report: &BaselineDeltaReport) -> String {
    let mut out = String::new();
    out.push_str("# SARAKURA baseline delta\n\n");
    out.push_str(&format!("- Status: `{}`\n", report.status));
    out.push_str(&format!("- Platform: `{}`\n", report.platform));
    out.push_str(&format!(
        "- Baseline build: `{}`\n",
        report.baseline_build_id.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- Current build: `{}`\n",
        report.current_build_id.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- Policy: new=`{}`, regression=`{}`\n\n",
        report.policy.fail_on_new, report.policy.fail_on_regression
    ));
    out.push_str("## Summary\n\n");
    out.push_str("| Metric | Count |\n|---|---:|\n");
    out.push_str(&format!(
        "| baseline_total | {} |\n",
        report.summary.baseline_total
    ));
    out.push_str(&format!(
        "| current_total | {} |\n",
        report.summary.current_total
    ));
    out.push_str(&format!("| new_total | {} |\n", report.summary.new_total));
    out.push_str(&format!(
        "| resolved_total | {} |\n",
        report.summary.resolved_total
    ));
    out.push_str(&format!(
        "| regressed_total | {} |\n",
        report.summary.regressed_total
    ));
    out.push_str(&format!(
        "| improved_total | {} |\n",
        report.summary.improved_total
    ));
    out.push_str(&format!(
        "| persisting_total | {} |\n",
        report.summary.persisting_total
    ));
    out.push_str("\n");
    push_section(&mut out, "New diagnostics", &report.new_diagnostics);
    push_section(
        &mut out,
        "Regressed diagnostics",
        &report.regressed_diagnostics,
    );
    push_section(
        &mut out,
        "Resolved diagnostics",
        &report.resolved_diagnostics,
    );
    push_section(
        &mut out,
        "Improved diagnostics",
        &report.improved_diagnostics,
    );
    push_section(
        &mut out,
        "Persisting diagnostics",
        &report.persisting_diagnostics,
    );
    out
}

fn push_section(out: &mut String, title: &str, items: &[BaselineDeltaItem]) {
    out.push_str(&format!("## {}\n\n", title));
    if items.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    out.push_str(
        "| Type | Severity | Count | Target | Source | Note |\n|---|---|---:|---|---|---|\n",
    );
    for item in items {
        let severity = item
            .current_severity
            .as_deref()
            .or(item.baseline_severity.as_deref())
            .unwrap_or("unknown");
        let count = item
            .current_event_count
            .or(item.baseline_event_count)
            .unwrap_or(0);
        let source = match (&item.primary_file, item.primary_line) {
            (Some(file), Some(line)) => format!("{}:{}", file, line),
            (Some(file), None) => file.clone(),
            _ => "-".to_string(),
        };
        out.push_str(&format!(
            "| `{}` | `{}` | {} | `{}` | `{}` | {} |\n",
            item.diagnostic_type, severity, count, item.repair_target, source, item.note
        ));
    }
    out.push_str("\n");
}

fn diagnostic_map<'a>(diagnostics: &'a [AiDiagnostic]) -> BTreeMap<String, &'a AiDiagnostic> {
    let mut map: BTreeMap<String, &'a AiDiagnostic> = BTreeMap::new();
    for diag in diagnostics {
        let key = diagnostic_key(diag);
        match map.get(&key) {
            Some(existing) if better_representative(*existing, diag) => {
                map.insert(key, diag);
            }
            None => {
                map.insert(key, diag);
            }
            _ => {}
        }
    }
    map
}

fn better_representative(existing: &AiDiagnostic, candidate: &AiDiagnostic) -> bool {
    severity_rank(&candidate.severity) > severity_rank(&existing.severity)
        || candidate.event_count > existing.event_count
        || candidate.confidence > existing.confidence
}

fn diagnostic_key(diag: &AiDiagnostic) -> String {
    format!(
        "{}|target={}|file={}|line={}|op={}",
        diag.diagnostic_type,
        diag.repair_target.target_type,
        diag.repair_target.primary_file.as_deref().unwrap_or("?"),
        diag.repair_target
            .primary_line
            .map(|x| x.to_string())
            .unwrap_or_else(|| "?".to_string()),
        diag.repair_target.op_id.as_deref().unwrap_or("?")
    )
}

fn delta_item(
    key: &str,
    baseline: Option<&AiDiagnostic>,
    current: Option<&AiDiagnostic>,
    note: &str,
) -> BaselineDeltaItem {
    let representative = current
        .or(baseline)
        .expect("baseline or current diagnostic must exist");
    BaselineDeltaItem {
        key: key.to_string(),
        diagnostic_type: representative.diagnostic_type.clone(),
        catalog_id: representative.catalog_id.clone(),
        repair_target: representative.repair_target.target_type.clone(),
        primary_file: representative.repair_target.primary_file.clone(),
        primary_line: representative.repair_target.primary_line,
        baseline_diagnostic_id: baseline.map(|d| d.diagnostic_id.clone()),
        current_diagnostic_id: current.map(|d| d.diagnostic_id.clone()),
        baseline_severity: baseline.map(|d| d.severity.clone()),
        current_severity: current.map(|d| d.severity.clone()),
        baseline_event_count: baseline.map(|d| d.event_count),
        current_event_count: current.map(|d| d.event_count),
        baseline_confidence: baseline.map(|d| d.confidence),
        current_confidence: current.map(|d| d.confidence),
        note: note.to_string(),
    }
}

fn sort_items(items: &mut [BaselineDeltaItem]) {
    items.sort_by(|a, b| {
        item_severity_rank(b)
            .cmp(&item_severity_rank(a))
            .then_with(|| item_event_count(b).cmp(&item_event_count(a)))
            .then_with(|| a.diagnostic_type.cmp(&b.diagnostic_type))
    });
}

fn item_severity_rank(item: &BaselineDeltaItem) -> u8 {
    item.current_severity
        .as_deref()
        .or(item.baseline_severity.as_deref())
        .map(severity_rank)
        .unwrap_or(0)
}

fn item_event_count(item: &BaselineDeltaItem) -> u64 {
    item.current_event_count
        .or(item.baseline_event_count)
        .unwrap_or(0)
}

fn severity_meets(severity: Option<&str>, threshold: &str) -> bool {
    let threshold_rank = severity_rank(threshold);
    if threshold_rank == 0 {
        return false;
    }
    severity.map(severity_rank).unwrap_or(0) >= threshold_rank
}

fn severity_rank(severity: &str) -> u8 {
    match normalize_severity(severity).as_str() {
        "error" => 3,
        "warn" => 2,
        "info" => 1,
        _ => 0,
    }
}

fn normalize_severity(severity: &str) -> String {
    match severity.to_ascii_lowercase().as_str() {
        "error" | "err" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "info" | "information" => "info".to_string(),
        "never" | "none" => "never".to_string(),
        _ => severity.to_ascii_lowercase(),
    }
}

fn normalize_fail_on(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "never" | "none" => "never".to_string(),
        "error" | "err" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "info" | "all" => "info".to_string(),
        _ => "error".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DiagnosticSummary, RepairTarget, RetestCondition, RomSummary, RunSummary, SourceMapping,
    };

    fn sample_doc(diagnostics: Vec<AiDiagnostic>) -> AiDiagnosticsDocument {
        let mut summary = DiagnosticSummary::default();
        summary.diagnostics_total = diagnostics.len();
        for diag in &diagnostics {
            match normalize_severity(&diag.severity).as_str() {
                "error" => summary.errors += 1,
                "warn" => summary.warnings += 1,
                _ => summary.infos += 1,
            }
        }
        AiDiagnosticsDocument {
            schema: "sarakura-gb-ai-diagnostics".to_string(),
            schema_version: 1,
            producer: "sarakura".to_string(),
            platform: "gb".to_string(),
            build_id: Some("b1".to_string()),
            rom: RomSummary {
                path: Some("game.gb".to_string()),
                target: Some("gbc".to_string()),
                hash: None,
            },
            run: RunSummary {
                frames_requested: 300,
                diagnostic_summary_limit: 200,
                events_loaded: diagnostics.len(),
                events_aggregated: diagnostics.len(),
                diagnostics_before_filter: diagnostics.len(),
                catalog_rules_total: 48,
                event_type_filter: vec![],
                phase_filter: vec![],
                pack_filter: vec![],
                min_severity: None,
            },
            summary,
            diagnostics,
        }
    }

    fn diag(id: &str, event_type: &str, severity: &str, count: u64) -> AiDiagnostic {
        AiDiagnostic {
            diagnostic_id: id.to_string(),
            catalog_id: Some("GBD001".to_string()),
            diagnostic_type: event_type.to_string(),
            severity: severity.to_string(),
            phase: Some("MVP-1".to_string()),
            category: Some("PPU/VRAM".to_string()),
            confidence: 0.9,
            source_mapping: Some(SourceMapping {
                source_file: Some("main.c".to_string()),
                source_line: Some(42),
                function_id: Some("func_draw".to_string()),
                function_name: Some("draw".to_string()),
                symbol_name: None,
                op_id: Some("op1".to_string()),
                pc: Some("0x415a".to_string()),
                bank: Some(1),
                prg_bank: None,
                mapping_confidence: 0.9,
            }),
            evidence_chain: vec![],
            repair_target: RepairTarget {
                target_type: "vblank_queue".to_string(),
                primary_file: Some("main.c".to_string()),
                primary_line: Some(42),
                function_id: Some("func_draw".to_string()),
                function_name: Some("draw".to_string()),
                op_id: Some("op1".to_string()),
                reason: "test".to_string(),
            },
            target_candidates: vec![],
            safe_patch_hint: "fix it".to_string(),
            retest_condition: RetestCondition {
                frames: 300,
                expect_absent: vec![event_type.to_string()],
                expect_not_worse: vec![],
                required_outputs: vec![],
            },
            event_count: count,
            first_seen: Some(1),
            last_seen: Some(2),
            snapshot_ref: None,
            trace_window_ref: None,
            sample_event_ids: vec!["evt_1".to_string()],
        }
    }

    #[test]
    fn delta_reports_new_and_resolved() {
        let baseline = sample_doc(vec![]);
        let current = sample_doc(vec![diag(
            "diag_1",
            "VRAM_WRITE_OUTSIDE_SAFE_PERIOD",
            "error",
            1,
        )]);
        let report = build_baseline_delta(&baseline, &current, "error", "error");
        assert_eq!(report.summary.new_total, 1);
        assert_eq!(report.status, "failed");
    }

    #[test]
    fn delta_reports_pass_for_same_document() {
        let baseline = sample_doc(vec![diag(
            "diag_1",
            "VRAM_WRITE_OUTSIDE_SAFE_PERIOD",
            "warn",
            1,
        )]);
        let current = sample_doc(vec![diag(
            "diag_2",
            "VRAM_WRITE_OUTSIDE_SAFE_PERIOD",
            "warn",
            1,
        )]);
        let report = build_baseline_delta(&baseline, &current, "error", "error");
        assert_eq!(report.summary.persisting_total, 1);
        assert_eq!(report.status, "passed");
        assert!(render_baseline_delta_markdown(&report).contains("Persisting diagnostics"));
    }
}

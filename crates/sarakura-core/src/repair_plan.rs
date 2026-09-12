use crate::model::{
    AiDiagnostic, AiDiagnosticsDocument, RepairPlan, RepairPlanStep, RepairPlanSummary,
    RetestCondition,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_repair_plan(doc: &AiDiagnosticsDocument) -> RepairPlan {
    let mut groups: BTreeMap<String, Vec<&AiDiagnostic>> = BTreeMap::new();
    for diagnostic in &doc.diagnostics {
        groups
            .entry(repair_group_key(diagnostic))
            .or_default()
            .push(diagnostic);
    }

    let mut steps: Vec<RepairPlanStep> = groups
        .into_values()
        .map(|diagnostics| build_step_from_group(&diagnostics))
        .collect();
    steps.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| b.event_count.cmp(&a.event_count))
            .then_with(|| b.max_confidence.total_cmp(&a.max_confidence))
            .then_with(|| a.repair_target_type.cmp(&b.repair_target_type))
    });
    for (idx, step) in steps.iter_mut().enumerate() {
        step.step_id = format!("repair_{:04}", idx + 1);
    }

    let mut summary = RepairPlanSummary::default();
    summary.total_steps = steps.len();
    summary.diagnostics_covered = doc.diagnostics.len();
    for step in &steps {
        match severity_rank(&step.severity) {
            0 => summary.error_steps += 1,
            1 => summary.warning_steps += 1,
            _ => summary.info_steps += 1,
        }
        *summary
            .target_groups
            .entry(step.repair_target_type.clone())
            .or_insert(0) += 1;
    }

    RepairPlan {
        schema: "sarakura-repair-plan".to_string(),
        schema_version: 1,
        platform: doc.platform.clone(),
        build_id: doc.build_id.clone(),
        generated_from: "ai_diagnostics.json".to_string(),
        summary,
        steps,
    }
}

pub fn render_repair_plan_markdown(plan: &RepairPlan) -> String {
    let mut out = String::new();
    out.push_str("# SARAKURA repair plan\n\n");
    out.push_str(&format!("- Platform: `{}`\n", plan.platform));
    out.push_str(&format!(
        "- Build ID: `{}`\n",
        plan.build_id.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!("- Steps: {}\n", plan.summary.total_steps));
    out.push_str(&format!(
        "- Severity groups: error={}, warn={}, info={}\n\n",
        plan.summary.error_steps, plan.summary.warning_steps, plan.summary.info_steps
    ));

    out.push_str("## Execution order\n\n");
    out.push_str("1. Fix `error` steps first.\n");
    out.push_str("2. Fix steps sharing the same `repair_target_type` together.\n");
    out.push_str("3. Re-run SARAKURA and confirm each `expect_absent` event disappeared.\n\n");

    for step in &plan.steps {
        out.push_str(&format!(
            "## {} `{}`\n\n",
            step.step_id, step.repair_target_type
        ));
        out.push_str(&format!("- Priority: `{}`\n", step.priority));
        out.push_str(&format!("- Severity: `{}`\n", step.severity));
        out.push_str(&format!(
            "- Diagnostics: `{}`\n",
            step.diagnostic_ids.join(", ")
        ));
        out.push_str(&format!("- Event count: `{}`\n", step.event_count));
        out.push_str(&format!("- Max confidence: `{:.2}`\n", step.max_confidence));
        if let Some(file) = &step.primary_file {
            out.push_str(&format!(
                "- Primary source: `{}`:{}\n",
                file,
                step.primary_line.unwrap_or(0)
            ));
        }
        if let Some(function_name) = &step.function_name {
            out.push_str(&format!("- Function: `{}`\n", function_name));
        }
        if let Some(op_id) = &step.op_id {
            out.push_str(&format!("- Operation ID: `{}`\n", op_id));
        }
        out.push_str("\nSafe patch hints:\n\n");
        for hint in &step.safe_patch_hints {
            out.push_str(&format!("- {}\n", hint));
        }
        out.push_str("\nRetest condition:\n\n");
        out.push_str(&format!("- Frames: `{}`\n", step.retest_condition.frames));
        out.push_str(&format!(
            "- Expect absent: `{}`\n",
            step.retest_condition.expect_absent.join(", ")
        ));
        if !step.evidence_refs.is_empty() {
            out.push_str("\nEvidence refs:\n\n");
            for ev in &step.evidence_refs {
                out.push_str(&format!("- `{}`\n", ev));
            }
        }
        out.push('\n');
    }

    out
}

fn repair_group_key(diagnostic: &AiDiagnostic) -> String {
    format!(
        "{}|file={}|line={}|func={}|op={}",
        diagnostic.repair_target.target_type,
        diagnostic
            .repair_target
            .primary_file
            .as_deref()
            .unwrap_or("?"),
        diagnostic
            .repair_target
            .primary_line
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".to_string()),
        diagnostic
            .repair_target
            .function_name
            .as_deref()
            .unwrap_or("?"),
        diagnostic.repair_target.op_id.as_deref().unwrap_or("?")
    )
}

fn build_step_from_group(diagnostics: &[&AiDiagnostic]) -> RepairPlanStep {
    let first = diagnostics[0];
    let severity = strongest_severity(diagnostics);
    let mut diagnostic_ids = BTreeSet::new();
    let mut diagnostic_types = BTreeSet::new();
    let mut catalog_ids = BTreeSet::new();
    let mut hints = BTreeSet::new();
    let mut evidence_refs = BTreeSet::new();
    let mut expect_absent = BTreeSet::new();
    let mut expect_not_worse = BTreeSet::new();
    let mut required_outputs = BTreeSet::new();
    let mut event_count = 0;
    let mut max_confidence = 0.0_f32;
    let mut frames = 0;

    for diagnostic in diagnostics {
        diagnostic_ids.insert(diagnostic.diagnostic_id.clone());
        diagnostic_types.insert(diagnostic.diagnostic_type.clone());
        if let Some(catalog_id) = &diagnostic.catalog_id {
            catalog_ids.insert(catalog_id.clone());
        }
        hints.insert(diagnostic.safe_patch_hint.clone());
        event_count += diagnostic.event_count;
        max_confidence = max_confidence.max(diagnostic.confidence);
        frames = frames.max(diagnostic.retest_condition.frames);
        for event_type in &diagnostic.retest_condition.expect_absent {
            expect_absent.insert(event_type.clone());
        }
        for event_type in &diagnostic.retest_condition.expect_not_worse {
            expect_not_worse.insert(event_type.clone());
        }
        for output in &diagnostic.retest_condition.required_outputs {
            required_outputs.insert(output.clone());
        }
        if let Some(snapshot_ref) = &diagnostic.snapshot_ref {
            evidence_refs.insert(snapshot_ref.clone());
        }
        if let Some(trace_window_ref) = &diagnostic.trace_window_ref {
            evidence_refs.insert(trace_window_ref.clone());
        }
        for event_id in &diagnostic.sample_event_ids {
            evidence_refs.insert(event_id.clone());
        }
    }

    RepairPlanStep {
        step_id: String::new(),
        priority: severity_rank(&severity),
        severity,
        repair_target_type: first.repair_target.target_type.clone(),
        primary_file: first.repair_target.primary_file.clone(),
        primary_line: first.repair_target.primary_line,
        function_id: first.repair_target.function_id.clone(),
        function_name: first.repair_target.function_name.clone(),
        op_id: first.repair_target.op_id.clone(),
        diagnostic_ids: diagnostic_ids.into_iter().collect(),
        diagnostic_types: diagnostic_types.into_iter().collect(),
        catalog_ids: catalog_ids.into_iter().collect(),
        event_count,
        max_confidence,
        safe_patch_hints: hints.into_iter().collect(),
        retest_condition: RetestCondition {
            frames,
            expect_absent: expect_absent.into_iter().collect(),
            expect_not_worse: expect_not_worse.into_iter().collect(),
            required_outputs: required_outputs.into_iter().collect(),
        },
        evidence_refs: evidence_refs.into_iter().collect(),
    }
}

fn strongest_severity(diagnostics: &[&AiDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| normalize_severity(&diagnostic.severity))
        .min_by_key(|severity| severity_rank(severity))
        .unwrap_or_else(|| "info".to_string())
}

fn normalize_severity(severity: &str) -> String {
    match severity.to_ascii_lowercase().as_str() {
        "error" | "err" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        _ => "info".to_string(),
    }
}

fn severity_rank(severity: &str) -> u32 {
    match normalize_severity(severity).as_str() {
        "error" => 0,
        "warn" => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagnosticSummary, RepairTarget, RomSummary, RunSummary, SourceMapping};

    fn sample_diag(id: &str, severity: &str, target: &str) -> AiDiagnostic {
        AiDiagnostic {
            diagnostic_id: id.to_string(),
            catalog_id: Some("GBD001".to_string()),
            diagnostic_type: "VRAM_WRITE_OUTSIDE_SAFE_PERIOD".to_string(),
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
                pc: Some("0x1234".to_string()),
                bank: Some(1),
                prg_bank: None,
                mapping_confidence: 0.9,
            }),
            evidence_chain: vec![],
            repair_target: RepairTarget {
                target_type: target.to_string(),
                primary_file: Some("main.c".to_string()),
                primary_line: Some(42),
                function_id: Some("func_draw".to_string()),
                function_name: Some("draw".to_string()),
                op_id: Some("op1".to_string()),
                reason: "test".to_string(),
            },
            target_candidates: vec![],
            safe_patch_hint: "move to queue".to_string(),
            retest_condition: RetestCondition {
                frames: 300,
                expect_absent: vec!["VRAM_WRITE_OUTSIDE_SAFE_PERIOD".to_string()],
                expect_not_worse: vec![],
                required_outputs: vec!["ai_diagnostics.json".to_string()],
            },
            event_count: 2,
            first_seen: Some(1),
            last_seen: Some(2),
            snapshot_ref: Some("snap.json".to_string()),
            trace_window_ref: None,
            sample_event_ids: vec!["evt_1".to_string()],
        }
    }

    fn sample_doc() -> AiDiagnosticsDocument {
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
                events_loaded: 2,
                events_aggregated: 2,
                diagnostics_before_filter: 2,
                catalog_rules_total: 48,
                event_type_filter: vec![],
                phase_filter: vec![],
                pack_filter: vec![],
                min_severity: None,
            },
            summary: DiagnosticSummary {
                diagnostics_total: 2,
                errors: 1,
                warnings: 1,
                infos: 0,
                ..DiagnosticSummary::default()
            },
            diagnostics: vec![
                sample_diag("diag_1", "error", "vblank_queue"),
                sample_diag("diag_2", "warn", "vblank_queue"),
            ],
        }
    }

    #[test]
    fn repair_plan_groups_by_repair_target() {
        let plan = build_repair_plan(&sample_doc());
        assert_eq!(plan.summary.total_steps, 1);
        assert_eq!(plan.summary.error_steps, 1);
        assert_eq!(plan.steps[0].diagnostic_ids.len(), 2);
        assert_eq!(plan.steps[0].event_count, 4);
    }

    #[test]
    fn markdown_contains_steps() {
        let plan = build_repair_plan(&sample_doc());
        let markdown = render_repair_plan_markdown(&plan);
        assert!(markdown.contains("SARAKURA repair plan"));
        assert!(markdown.contains("repair_0001"));
    }
}

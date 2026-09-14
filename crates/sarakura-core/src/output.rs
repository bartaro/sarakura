use crate::model::AiDiagnosticsDocument;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

// Write the complete structured diagnostic document as pretty JSON, creating
// parent directories and reporting serialization or write failures with context.
pub fn write_ai_diagnostics(path: impl AsRef<Path>, doc: &AiDiagnosticsDocument) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output dir: {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(doc).context("failed to serialize ai diagnostics")?;
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

// Write only the summary counts as pretty JSON for lightweight downstream consumers.
pub fn write_diagnostic_summary(path: impl AsRef<Path>, doc: &AiDiagnosticsDocument) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output dir: {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&doc.summary)
        .context("failed to serialize diagnostic summary")?;
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

// Render an English Markdown repair brief with evidence, candidate targets and
// retest conditions. This creates a report; it does not apply patches or execute tests.
pub fn write_repair_prompt(path: impl AsRef<Path>, doc: &AiDiagnosticsDocument) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output dir: {}", parent.display()))?;
    }
    let mut out = String::new();
    out.push_str("# SARAKURA repair prompt\n\n");
    out.push_str("This summary is intended for an AI repair agent or a human maintainer.\n\n");
    out.push_str(&format!("- Platform: `{}`\n", doc.platform));
    out.push_str(&format!(
        "- Build ID: `{}`\n",
        doc.build_id.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- ROM: `{}`\n",
        doc.rom.path.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- Events loaded / aggregated: {} / {}\n",
        doc.run.events_loaded, doc.run.events_aggregated
    ));
    out.push_str(&format!(
        "- Diagnostics: total={}, errors={}, warnings={}, infos={}, unmapped={}\n\n",
        doc.summary.diagnostics_total,
        doc.summary.errors,
        doc.summary.warnings,
        doc.summary.infos,
        doc.summary.unmapped_diagnostics
    ));

    out.push_str("## Repair order\n\n");
    out.push_str("1. Fix `error` diagnostics first.\n");
    out.push_str("2. Address diagnostics sharing the same `repair_target.target_type` together.\n");
    out.push_str(
        "3. After the fix, verify that the event types in `retest_condition.expect_absent` no longer occur.\n\n",
    );

    out.push_str("## Diagnostics\n\n");
    // Preserve diagnostic order. Many absent labels render as unknown; a present
    // file with no line renders line 0 as a display fallback, not a real location.
    // Embedded labels/hints are interpolated without general Markdown escaping.
    for diag in &doc.diagnostics {
        out.push_str(&format!(
            "### {} `{}`\n\n",
            diag.diagnostic_id, diag.diagnostic_type
        ));
        out.push_str(&format!(
            "- catalog_id: `{}`\n",
            diag.catalog_id.as_deref().unwrap_or("unknown")
        ));
        out.push_str(&format!("- severity: `{}`\n", diag.severity));
        out.push_str(&format!(
            "- phase/category: `{}` / `{}`\n",
            diag.phase.as_deref().unwrap_or("unknown"),
            diag.category.as_deref().unwrap_or("unknown")
        ));
        out.push_str(&format!("- confidence: {:.2}\n", diag.confidence));
        out.push_str(&format!("- event_count: {}\n", diag.event_count));
        out.push_str(&format!(
            "- repair_target: `{}`\n",
            diag.repair_target.target_type
        ));
        if let Some(file) = &diag.repair_target.primary_file {
            out.push_str(&format!(
                "- primary_file: `{}`:{}\n",
                file,
                diag.repair_target.primary_line.unwrap_or(0)
            ));
        }
        if let Some(function) = &diag.repair_target.function_name {
            out.push_str(&format!("- function: `{}`\n", function));
        } else if let Some(function_id) = &diag.repair_target.function_id {
            out.push_str(&format!("- function_id: `{}`\n", function_id));
        }
        if let Some(op_id) = &diag.repair_target.op_id {
            out.push_str(&format!("- op_id: `{}`\n", op_id));
        }
        if let Some(snapshot) = &diag.snapshot_ref {
            out.push_str(&format!("- snapshot_ref: `{}`\n", snapshot));
        }
        if let Some(trace) = &diag.trace_window_ref {
            out.push_str(&format!("- trace_window_ref: `{}`\n", trace));
        }
        out.push_str(&format!("- safe_patch_hint: {}\n", diag.safe_patch_hint));
        out.push_str(&format!(
            "- retest: frames={}, expect_absent={:?}\n\n",
            diag.retest_condition.frames, diag.retest_condition.expect_absent
        ));
        if !diag.target_candidates.is_empty() {
            out.push_str("Target candidates:\n\n");
            for c in &diag.target_candidates {
                out.push_str(&format!(
                    "- {}. `{}` — {}\n",
                    c.rank, c.target_type, c.reason
                ));
            }
            out.push('\n');
        }
        out.push_str("Evidence chain:\n\n");
        for ev in &diag.evidence_chain {
            out.push_str(&format!("- `{}`", ev.kind));
            if let Some(id) = &ev.event_id {
                out.push_str(&format!(" event_id=`{}`", id));
            }
            if let Some(pc) = &ev.pc {
                out.push_str(&format!(" pc=`{}`", pc));
            }
            if let Some(op) = &ev.op_id {
                out.push_str(&format!(" op_id=`{}`", op));
            }
            if let Some(note) = &ev.note {
                out.push_str(&format!(" — {}", note));
            }
            out.push('\n');
        }
        out.push('\n');
    }
    fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

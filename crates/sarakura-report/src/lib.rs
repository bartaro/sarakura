use anyhow::{Context, Result};
use sarakura_core::{
    build_automation_plan, build_repair_plan, build_retest_plan, render_automation_plan_markdown,
    render_repair_plan_markdown, AiDiagnosticsDocument,
};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::FileOptions;

// Render an English HTML document from the supplied diagnostics without re-running
// analysis or redaction. Encode dynamic text before adding the surrounding markup.
pub fn render_html(doc: &AiDiagnosticsDocument) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    html.push_str("<title>SARAKURA Report</title>");
    html.push_str("<style>body{font-family:system-ui,sans-serif;margin:24px;line-height:1.6}table{border-collapse:collapse;width:100%;margin:16px 0}td,th{border:1px solid #ddd;padding:6px;vertical-align:top}th{background:#f4f4f4}.error{color:#b00020;font-weight:700}.warn{color:#9a6700;font-weight:700}.info{color:#175a9c}.muted{color:#666}code{background:#f6f8fa;padding:2px 4px;border-radius:4px}.card{border:1px solid #ddd;border-radius:8px;padding:12px;margin:12px 0}.small{font-size:0.9em}</style>");
    html.push_str("</head><body>");
    html.push_str(&format!(
        "<h1>SARAKURA Report: {}</h1>",
        escape(&doc.platform)
    ));
    html.push_str(&format!(
        "<p>Build ID: <code>{}</code></p>",
        escape(doc.build_id.as_deref().unwrap_or("unknown"))
    ));
    html.push_str(&format!(
        "<p>ROM: <code>{}</code> / target: <code>{}</code> / hash: <code>{}</code></p>",
        escape(doc.rom.path.as_deref().unwrap_or("unknown")),
        escape(doc.rom.target.as_deref().unwrap_or("unknown")),
        escape(doc.rom.hash.as_deref().unwrap_or("unknown"))
    ));
    html.push_str(&format!(
        "<p>Total: {} / errors: {} / warnings: {} / infos: {} / unmapped: {}</p>",
        doc.summary.diagnostics_total,
        doc.summary.errors,
        doc.summary.warnings,
        doc.summary.infos,
        doc.summary.unmapped_diagnostics
    ));
    html.push_str(&format!("<p class=\"muted small\">events loaded: {} / aggregated: {} / catalog rules: {} / before filter: {}</p>",
        doc.run.events_loaded, doc.run.events_aggregated, doc.run.catalog_rules_total, doc.run.diagnostics_before_filter));

    html.push_str("<h2>Breakdown</h2><table><thead><tr><th>Phase</th><th>Count</th><th>Category</th><th>Count</th><th>Repair target</th><th>Count</th></tr></thead><tbody>");
    // Display three independently sorted breakdowns side by side; entries on
    // the same row do not imply a relationship. Shorter lists receive blank cells.
    let max_rows = *[
        doc.summary.phases.len(),
        doc.summary.categories.len(),
        doc.summary.repair_targets.len(),
    ]
    .iter()
    .max()
    .unwrap_or(&0);
    let phases: Vec<_> = doc.summary.phases.iter().collect();
    let categories: Vec<_> = doc.summary.categories.iter().collect();
    let targets: Vec<_> = doc.summary.repair_targets.iter().collect();
    for i in 0..max_rows {
        let (p, pc) = phases
            .get(i)
            .map(|(k, v)| (k.as_str(), **v))
            .unwrap_or(("", 0));
        let (c, cc) = categories
            .get(i)
            .map(|(k, v)| (k.as_str(), **v))
            .unwrap_or(("", 0));
        let (t, tc) = targets
            .get(i)
            .map(|(k, v)| (k.as_str(), **v))
            .unwrap_or(("", 0));
        html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
            escape(p), pc, escape(c), cc, escape(t), tc));
    }
    html.push_str("</tbody></table>");

    html.push_str("<h2>Diagnostics</h2><table><thead><tr><th>ID</th><th>Type</th><th>Severity</th><th>Confidence</th><th>Count</th><th>Source</th><th>Repair Target</th><th>Hint</th><th>Evidence refs</th></tr></thead><tbody>");
    for d in &doc.diagnostics {
        let class = if d.severity == "error" {
            "error"
        } else if d.severity == "warn" || d.severity == "warning" {
            "warn"
        } else {
            "info"
        };
        let source = d
            .source_mapping
            .as_ref()
            .and_then(|m| {
                m.source_file.as_ref().map(|f| match m.source_line {
                    Some(line) => format!("{}:{}", f, line),
                    None => f.clone(),
                })
            })
            .unwrap_or_else(|| "-".to_string());
        // Escape each reference as display text before inserting the line separator.
        // Escaping only the joined string would also hide the intentional <br> markup.
        let refs = [d.snapshot_ref.as_deref(), d.trace_window_ref.as_deref()]
            .into_iter()
            .flatten()
            .map(escape)
            .collect::<Vec<_>>()
            .join("<br>");
        html.push_str(&format!(
            "<tr><td>{}</td><td><code>{}</code><br><span class=\"small muted\">{}</span></td><td class=\"{}\">{}</td><td>{:.2}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
            escape(&d.diagnostic_id), escape(&d.diagnostic_type), escape(d.catalog_id.as_deref().unwrap_or("")), class, escape(&d.severity), d.confidence,
            d.event_count, escape(&source), escape(&d.repair_target.target_type), escape(&d.safe_patch_hint), refs
        ));
    }
    html.push_str("</tbody></table></body></html>");
    html
}

// Create the parent directory and replace the report file with rendered UTF-8 HTML.
// Write failures carry the destination path; the write is not atomic.
pub fn write_html_report(path: impl AsRef<Path>, doc: &AiDiagnosticsDocument) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, render_html(doc))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

// Compare stored totals and sets of diagnostic types in English Markdown. A type
// is resolved only when absent from the later document; source locations and
// per-instance regressions are not compared here. Callers must choose comparable runs.
pub fn compare_documents(before: &AiDiagnosticsDocument, after: &AiDiagnosticsDocument) -> String {
    let mut md = String::new();
    md.push_str("# SARAKURA before/after comparison\n\n");
    md.push_str("| Metric | Before | After | Delta |\n|---|---:|---:|---:|\n");
    macro_rules! row {
        ($name:expr, $b:expr, $a:expr) => {{
            let b = $b as i64;
            let a = $a as i64;
            md.push_str(&format!("| {} | {} | {} | {} |\n", $name, b, a, a - b));
        }};
    }
    row!(
        "total",
        before.summary.diagnostics_total,
        after.summary.diagnostics_total
    );
    row!("errors", before.summary.errors, after.summary.errors);
    row!("warnings", before.summary.warnings, after.summary.warnings);
    row!("infos", before.summary.infos, after.summary.infos);
    row!(
        "unmapped",
        before.summary.unmapped_diagnostics,
        after.summary.unmapped_diagnostics
    );

    // Deduplicate type names, so count changes within a surviving type do not
    // appear as newly introduced or resolved types.
    let before_types: BTreeSet<_> = before
        .diagnostics
        .iter()
        .map(|d| d.diagnostic_type.clone())
        .collect();
    let after_types: BTreeSet<_> = after
        .diagnostics
        .iter()
        .map(|d| d.diagnostic_type.clone())
        .collect();
    let resolved: Vec<_> = before_types.difference(&after_types).cloned().collect();
    let new: Vec<_> = after_types.difference(&before_types).cloned().collect();

    md.push_str("\n## Resolved diagnostic types\n\n");
    if resolved.is_empty() {
        md.push_str("- none\n");
    } else {
        for t in resolved {
            md.push_str(&format!("- `{}`\n", t));
        }
    }
    md.push_str("\n## Newly introduced diagnostic types\n\n");
    if new.is_empty() {
        md.push_str("- none\n");
    } else {
        for t in new {
            md.push_str(&format!("- `{}`\n", t));
        }
    }
    md.push_str("\n## Remaining diagnostics\n\n");
    for d in &after.diagnostics {
        md.push_str(&format!(
            "- `{}` {} confidence={:.2} target=`{}`\n",
            d.diagnostic_type, d.severity, d.confidence, d.repair_target.target_type
        ));
    }
    md
}

// Package derived reports and plans in a deflated ZIP. Original input paths are
// intentionally ignored; no ROM, source file, snapshot or trace is copied. The
// supplied document is serialized as-is, so apply label redaction before this call
// when needed. Errors can leave a partial archive at the destination.
pub fn write_repro_bundle(
    out_zip: impl AsRef<Path>,
    _input_paths: &[PathBuf],
    doc: &AiDiagnosticsDocument,
) -> Result<()> {
    let out_zip = out_zip.as_ref();
    if let Some(parent) = out_zip.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let file =
        File::create(out_zip).with_context(|| format!("failed to create {}", out_zip.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let ai = serde_json::to_vec_pretty(doc).context("failed to serialize ai diagnostics")?;
    zip.start_file("ai_diagnostics.json", options)?;
    zip.write_all(&ai)?;

    let diagnostic_summary = serde_json::to_vec_pretty(&doc.summary)
        .context("failed to serialize diagnostic summary")?;
    zip.start_file("diagnostic_summary.json", options)?;
    zip.write_all(&diagnostic_summary)?;

    let report = render_html(doc);
    zip.start_file("report.html", options)?;
    zip.write_all(report.as_bytes())?;

    let retest_plan = build_retest_plan(doc);
    zip.start_file("retest_plan.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&retest_plan)?.as_bytes())?;

    let repair_plan = build_repair_plan(doc);
    zip.start_file("repair_plan.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&repair_plan)?.as_bytes())?;
    zip.start_file("repair_plan.md", options)?;
    zip.write_all(render_repair_plan_markdown(&repair_plan).as_bytes())?;

    // Regenerate the default command proposal for this bundle; the archive
    // contains descriptions and commands, not evidence that those commands ran.
    let automation_plan = build_automation_plan(doc, None);
    zip.start_file("automation_plan.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&automation_plan)?.as_bytes())?;
    zip.start_file("automation_plan.md", options)?;
    zip.write_all(render_automation_plan_markdown(&automation_plan).as_bytes())?;

    let manifest = serde_json::json!({
        "schema": "sarakura-repro-bundle-manifest",
        "schema_version": 1,
        "producer": "sarakura",
        "platform": &doc.platform,
        "rom": {
            "path": &doc.rom.path,
            "hash": &doc.rom.hash,
            "target": &doc.rom.target,
        },
        "build_id": &doc.build_id,
        "diagnostics_total": doc.summary.diagnostics_total,
        "diagnostics": "ai_diagnostics.json",
        "diagnostic_summary": "diagnostic_summary.json",
        "report": "report.html",
        "retest_plan": "retest_plan.json",
        "repair_plan": "repair_plan.json",
        "repair_plan_markdown": "repair_plan.md",
        "automation_plan": "automation_plan.json",
        "automation_plan_markdown": "automation_plan.md",
        "inputs": [],
        "source_inputs_omitted": true
    });
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    // Write the ZIP central directory and propagate finalization errors.
    zip.finish()?;
    Ok(())
}

// Encode HTML text and double-quoted attribute delimiters, replacing ampersands
// first so the generated entities are not encoded again. This is not URL encoding.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

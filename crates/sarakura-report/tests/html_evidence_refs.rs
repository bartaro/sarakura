use sarakura_core::{
    AiDiagnosticsDocument, AnalyzeInput, AnalyzeOptions, BuildMetadata, DiagnosticEvent,
};
use sarakura_report::render_html;

// Keep project labels visible so this test exercises HTML encoding rather than redaction.
fn document(snapshot: Option<&str>, trace: Option<&str>) -> AiDiagnosticsDocument {
    let event: DiagnosticEvent = serde_json::from_value(serde_json::json!({
        "event_type": "TEST_EVIDENCE_REFERENCE",
        "snapshot_ref": snapshot,
        "trace_window_ref": trace
    }))
    .unwrap();
    sarakura_core::analyze(AnalyzeInput {
        metadata: BuildMetadata::default(),
        events: vec![event],
        catalog: vec![],
        options: AnalyzeOptions {
            allow_project_labels: true,
            ..AnalyzeOptions::default()
        },
    })
}

#[test]
// References are display text. Keep the separator markup, but never interpret names as HTML.
fn escapes_each_evidence_reference_before_inserting_line_breaks() {
    let html = render_html(&document(
        Some("<img src=x onerror=alert(1)>.json"),
        Some("trace&\"<frame>.jsonl"),
    ));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;.json<br>trace&amp;&quot;&lt;frame&gt;.jsonl"));
    assert!(!html.contains("<img"));
    assert!(!html.contains("<frame>"));
}

#[test]
// Missing references contribute no placeholder or extra separator, and no encoding is repeated.
fn handles_missing_references_and_encodes_literal_entity_text_once() {
    let one = render_html(&document(None, Some("capture &amp; trace.jsonl")));
    assert!(one.contains("<td>capture &amp;amp; trace.jsonl</td>"));
    let none = render_html(&document(None, None));
    assert!(none.contains("<td></td></tr>"));
}

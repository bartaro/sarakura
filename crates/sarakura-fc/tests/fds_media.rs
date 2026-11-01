use sarakura_core::{BuildMetadata, DiagnosticEvent};
#[test]
fn fds_media_failures_have_concrete_repair_guidance() {
    let events: Vec<DiagnosticEvent> = serde_json::from_str(
        r#"[
        {"event_type":"FDS_MEDIA_CRC_ERROR","frame":12,"addr":"0x4030"},
        {"event_type":"FDS_MEDIA_WRITE_PROTECTED","frame":13,"addr":"0x4024"}
    ]"#,
    )
    .unwrap();
    let report = sarakura_fc::analyze_fc(BuildMetadata::default(), events, 1200, 10).unwrap();
    assert_eq!(report.diagnostics.len(), 2);
    for diagnostic in &report.diagnostics {
        assert!(diagnostic.catalog_id.is_some());
        assert_eq!(diagnostic.category.as_deref(), Some("FDS/Media"));
        assert!(!diagnostic.safe_patch_hint.contains("No SARAKURA catalog"));
    }
}

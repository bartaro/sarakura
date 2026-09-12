use crate::model::DiagnosticRule;
use anyhow::{Context, Result};
use std::collections::BTreeMap;

pub fn load_catalog_from_str(s: &str) -> Result<Vec<DiagnosticRule>> {
    let rules: Vec<DiagnosticRule> =
        serde_json::from_str(s).context("failed to parse diagnostic catalog json")?;
    Ok(rules)
}

pub fn catalog_by_event_type(rules: &[DiagnosticRule]) -> BTreeMap<String, DiagnosticRule> {
    rules
        .iter()
        .cloned()
        .map(|r| (r.event_type.clone(), r))
        .collect()
}

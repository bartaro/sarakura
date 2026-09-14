use crate::model::DiagnosticRule;
use anyhow::{Context, Result};
use std::collections::BTreeMap;

// Deserialize the rule array and attach parse context. This does not check
// rule-ID uniqueness, emitter support or whether detection prose is executable.
pub fn load_catalog_from_str(s: &str) -> Result<Vec<DiagnosticRule>> {
    let rules: Vec<DiagnosticRule> =
        serde_json::from_str(s).context("failed to parse diagnostic catalog json")?;
    Ok(rules)
}

// Clone rules into a sorted exact-name lookup. If several rules have the same
// event_type, the last input rule replaces earlier ones rather than forming a list.
pub fn catalog_by_event_type(rules: &[DiagnosticRule]) -> BTreeMap<String, DiagnosticRule> {
    rules
        .iter()
        .cloned()
        .map(|r| (r.event_type.clone(), r))
        .collect()
}

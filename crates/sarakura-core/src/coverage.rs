use crate::model::{
    CatalogCoverageReport, CatalogCoverageRow, DiagnosticEvent, DiagnosticRule, Platform,
    UnknownEventSuggestion,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_catalog_coverage(
    platform: Platform,
    catalog: &[DiagnosticRule],
    events: &[DiagnosticEvent],
) -> CatalogCoverageReport {
    let mut event_counts: BTreeMap<String, u64> = BTreeMap::new();
    for event in events {
        *event_counts.entry(event.event_type.clone()).or_insert(0) += event.normalized_count();
    }

    let known_types: BTreeSet<_> = catalog.iter().map(|r| r.event_type.clone()).collect();
    let unknown_event_types = event_counts
        .keys()
        .filter(|event_type| !known_types.contains(*event_type))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_event_suggestions = unknown_event_types
        .iter()
        .map(|event_type| suggest_catalog_candidates(event_type, catalog))
        .collect::<Vec<_>>();

    let rules = catalog
        .iter()
        .map(|rule| {
            let event_count = event_counts.get(&rule.event_type).copied().unwrap_or(0);
            CatalogCoverageRow {
                id: rule.id.clone(),
                event_type: rule.event_type.clone(),
                phase: rule.phase.clone(),
                severity: rule.severity.clone(),
                category: rule.category.clone(),
                observed: event_count > 0,
                event_count,
            }
        })
        .collect::<Vec<_>>();

    let observed_catalog_rules = rules.iter().filter(|row| row.observed).count();
    CatalogCoverageReport {
        schema: "sarakura-catalog-coverage".to_string(),
        schema_version: 1,
        platform: platform.as_str().to_string(),
        catalog_rules_total: catalog.len(),
        observed_catalog_rules,
        missing_catalog_rules: catalog.len().saturating_sub(observed_catalog_rules),
        unknown_event_types,
        unknown_event_suggestions,
        event_types_seen: event_counts,
        rules,
    }
}

fn suggest_catalog_candidates(
    event_type: &str,
    catalog: &[DiagnosticRule],
) -> UnknownEventSuggestion {
    let event_tokens = tokenize(event_type);
    let mut scored = catalog
        .iter()
        .map(|rule| {
            let mut score = 0usize;
            let rule_tokens = tokenize(&rule.event_type);
            for token in &event_tokens {
                if rule_tokens.contains(token) {
                    score += token.len().max(1);
                }
            }
            (score, rule)
        })
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    let top = scored.into_iter().take(3).collect::<Vec<_>>();
    let candidate_catalog_ids: Vec<String> = top.iter().map(|(_, rule)| rule.id.clone()).collect();
    let candidate_event_types: Vec<String> = top
        .iter()
        .map(|(_, rule)| rule.event_type.clone())
        .collect();
    let note = if candidate_event_types.is_empty() {
        "No close catalog match found; add an alias or a new catalog rule if this event is intentional."
            .to_string()
    } else {
        "Closest catalog entries were selected by shared event_type tokens; add an alias if this is an emitter spelling change."
            .to_string()
    };
    UnknownEventSuggestion {
        event_type: event_type.to_string(),
        candidate_catalog_ids,
        candidate_event_types,
        note,
    }
}

fn tokenize(value: &str) -> BTreeSet<String> {
    value
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagnosticEvent, DiagnosticRule, FixturePair};
    use serde_json::Value;
    use std::collections::BTreeMap;

    fn rule(id: &str, event_type: &str) -> DiagnosticRule {
        DiagnosticRule {
            id: id.to_string(),
            event_type: event_type.to_string(),
            phase: "MVP-1".to_string(),
            severity: "error".to_string(),
            category: "PPU/VRAM".to_string(),
            detection_condition: String::new(),
            required_metadata: Vec::new(),
            event_fields: Vec::new(),
            repair_target_candidates: Vec::new(),
            safe_patch_hint: String::new(),
            fixtures: FixturePair::default(),
            codex_tasks: Vec::new(),
        }
    }

    fn event(event_type: &str) -> DiagnosticEvent {
        DiagnosticEvent {
            schema: None,
            schema_version: None,
            event_id: None,
            event_type: event_type.to_string(),
            severity: None,
            frame: None,
            scanline: None,
            dot: None,
            cpu_cycle: None,
            pc: None,
            bank: None,
            prg_bank: None,
            addr: None,
            value: None,
            function_id_guess: None,
            symbol_hint: None,
            function_hint: None,
            op_id_guess: None,
            summary_key: None,
            count: Some(2),
            first_seen: None,
            last_seen: None,
            snapshot_ref: None,
            trace_window_ref: None,
            extra: BTreeMap::<String, Value>::new(),
        }
    }

    #[test]
    fn reports_catalog_coverage_and_unknown_events() {
        let report = build_catalog_coverage(
            Platform::Gb,
            &[rule("GBD001", "A"), rule("GBD002", "B")],
            &[event("A"), event("UNKNOWN")],
        );
        assert_eq!(report.catalog_rules_total, 2);
        assert_eq!(report.observed_catalog_rules, 1);
        assert_eq!(report.missing_catalog_rules, 1);
        assert_eq!(report.event_types_seen.get("A"), Some(&2));
        assert_eq!(report.unknown_event_types, vec!["UNKNOWN".to_string()]);
        assert_eq!(report.unknown_event_suggestions.len(), 1);
    }
}

use crate::analyze::severity_rank;
use crate::model::DiagnosticEvent;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn normalize_events(events: &[DiagnosticEvent]) -> Vec<DiagnosticEvent> {
    let mut by_key: BTreeMap<String, DiagnosticEvent> = BTreeMap::new();
    for event in events {
        let key = event.stable_key();
        if let Some(existing) = by_key.get_mut(&key) {
            merge_event(existing, event);
        } else {
            let mut cloned = event.clone();
            let first = cloned.first_seen.or(cloned.frame);
            let last = cloned.last_seen.or(cloned.frame);
            cloned.first_seen = first;
            cloned.last_seen = last;
            cloned.count = Some(cloned.normalized_count());
            cloned.severity = Some(normalize_severity(
                cloned.severity.as_deref().unwrap_or("warn"),
            ));
            by_key.insert(key, cloned);
        }
    }
    by_key.into_values().collect()
}

pub fn write_events_jsonl(path: impl AsRef<Path>, events: &[DiagnosticEvent]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output dir: {}", parent.display()))?;
    }
    let mut out = String::new();
    for event in events {
        out.push_str(&serde_json::to_string(event).context("failed to serialize event")?);
        out.push('\n');
    }
    fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))
}

fn merge_event(existing: &mut DiagnosticEvent, incoming: &DiagnosticEvent) {
    let count = existing.normalized_count() + incoming.normalized_count();
    existing.count = Some(count);
    existing.first_seen = min_opt(
        existing.first_seen.or(existing.frame),
        incoming.first_seen.or(incoming.frame),
    );
    existing.last_seen = max_opt(
        existing.last_seen.or(existing.frame),
        incoming.last_seen.or(incoming.frame),
    );
    existing.frame = existing.first_seen;
    if stronger_severity(incoming.severity.as_deref(), existing.severity.as_deref()) {
        existing.severity = incoming.severity.as_deref().map(normalize_severity);
    }
    if existing.event_id.is_none() {
        existing.event_id = incoming.event_id.clone();
    }
    if existing.snapshot_ref.is_none() {
        existing.snapshot_ref = incoming.snapshot_ref.clone();
    }
    if existing.trace_window_ref.is_none() {
        existing.trace_window_ref = incoming.trace_window_ref.clone();
    }
    if existing.function_id_guess.is_none() {
        existing.function_id_guess = incoming.function_id_guess.clone();
    }
    if existing.op_id_guess.is_none() {
        existing.op_id_guess = incoming.op_id_guess.clone();
    }
}

fn stronger_severity(incoming: Option<&str>, existing: Option<&str>) -> bool {
    severity_rank(&normalize_severity(incoming.unwrap_or("warn")))
        > severity_rank(&normalize_severity(existing.unwrap_or("warn")))
}

fn normalize_severity(severity: &str) -> String {
    match severity.to_ascii_lowercase().as_str() {
        "error" | "err" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "info" | "note" => "info".to_string(),
        _ => "warn".to_string(),
    }
}

fn min_opt(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn max_opt(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeMap;

    fn event(id: &str, frame: u64, severity: &str, count: Option<u64>) -> DiagnosticEvent {
        DiagnosticEvent {
            schema: None,
            schema_version: None,
            event_id: Some(id.to_string()),
            event_type: "VRAM_WRITE_OUTSIDE_SAFE_PERIOD".to_string(),
            severity: Some(severity.to_string()),
            frame: Some(frame),
            scanline: None,
            dot: None,
            cpu_cycle: None,
            pc: Some("0x1234".to_string()),
            bank: Some(1),
            prg_bank: None,
            addr: Some("0x9800".to_string()),
            value: None,
            function_id_guess: None,
            symbol_hint: None,
            function_hint: None,
            op_id_guess: None,
            summary_key: Some("same".to_string()),
            count,
            first_seen: None,
            last_seen: None,
            snapshot_ref: None,
            trace_window_ref: None,
            extra: BTreeMap::<String, Value>::new(),
        }
    }

    #[test]
    fn merges_duplicate_events() {
        let events = vec![
            event("a", 10, "warn", Some(2)),
            event("b", 20, "error", Some(3)),
        ];
        let normalized = normalize_events(&events);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].count, Some(5));
        assert_eq!(normalized[0].severity.as_deref(), Some("error"));
        assert_eq!(normalized[0].first_seen, Some(10));
        assert_eq!(normalized[0].last_seen, Some(20));
    }
}

use crate::model::DiagnosticEvent;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Load diagnostic events from JSONL.
///
/// v0.2 also accepts a small JSON array for tests and hand-written fixtures. The
/// production emitter format remains JSONL so KOKURA/KUROSAKI never need to keep
/// all events in memory.
pub fn load_events_jsonl(path: impl AsRef<Path>) -> Result<Vec<DiagnosticEvent>> {
    let path = path.as_ref();
    // Inspect the full text to detect array fixtures, then reopen JSONL for line-based
    // parsing. The loader still materializes the input and result; it is not streaming end to end.
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to open events jsonl: {}", path.display()))?;
    let trimmed = text.trim_start();
    if trimmed.starts_with('[') {
        let events: Vec<DiagnosticEvent> = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "failed to parse diagnostic event json array: {}",
                path.display()
            )
        })?;
        return Ok(events);
    }

    let file = File::open(path)
        .with_context(|| format!("failed to open events jsonl: {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    // Skip blank/comment lines and attach a one-based input line number to parse errors.
    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("failed to read line {}", idx + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let event: DiagnosticEvent = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "failed to parse diagnostic event at {}:{}",
                path.display(),
                idx + 1
            )
        })?;
        out.push(event);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    // Verify a single JSONL event can be loaded with its event type preserved.
    fn loads_jsonl_events() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            "{{\"event_type\":\"VRAM_WRITE_OUTSIDE_SAFE_PERIOD\",\"event_id\":\"evt_1\"}}"
        )
        .unwrap();
        let events = load_events_jsonl(tmp.path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "VRAM_WRITE_OUTSIDE_SAFE_PERIOD");
    }

    #[test]
    // Verify the compact JSON-array fixture format is accepted alongside production JSONL.
    fn loads_json_array_fixture() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            "[{{\"event_type\":\"PPU_WRITE_OUTSIDE_VBLANK\",\"event_id\":\"evt_1\"}}]"
        )
        .unwrap();
        let events = load_events_jsonl(tmp.path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "PPU_WRITE_OUTSIDE_VBLANK");
    }
}

use crate::model::{DiagnosticEvent, DiagnosticRule, SourceMapping};

// Compute a bounded heuristic score from available event identifiers and source
// mapping evidence. This is not a calibrated probability of correctness, and the
// rule argument is currently unused. Missing evidence leaves the base score intact.
pub fn score_confidence(
    _rule: Option<&DiagnosticRule>,
    event: &DiagnosticEvent,
    source_mapping: Option<&SourceMapping>,
) -> f32 {
    let mut score: f32 = 0.50;

    if event.event_id.is_some() {
        score += 0.05;
    }
    if event.pc.is_some() {
        score += 0.07;
    }
    if event.op_id_guess.is_some() {
        score += 0.10;
    }
    if event.function_id_guess.is_some() || event.function_hint.is_some() {
        score += 0.06;
    }
    // Weight mapping confidence and reward concrete file/line and operation references
    // without treating a guessed source location as independently verified execution.
    if let Some(mapping) = source_mapping {
        score += mapping.mapping_confidence * 0.20;
        if mapping.source_file.is_some() && mapping.source_line.is_some() {
            score += 0.08;
        }
        if mapping.op_id.is_some() {
            score += 0.05;
        }
    }

    score.clamp(0.10, 0.99)
}

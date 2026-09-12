use crate::model::{DiagnosticEvent, DiagnosticRule, SourceMapping};

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

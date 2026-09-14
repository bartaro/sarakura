use crate::model::{DiagnosticPack, DiagnosticPackPlan, DiagnosticRule, Platform};
use std::collections::BTreeMap;

// Group catalog IDs by the original phase and the first category component.
// Sorted map keys make groups stable; ID order within each group follows the
// catalog. Recommendations are a selected subset, not every possible group.
pub fn build_pack_plan(platform: Platform, catalog: &[DiagnosticRule]) -> DiagnosticPackPlan {
    let mut phases: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut domains: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for rule in catalog {
        phases
            .entry(rule.phase.clone())
            .or_default()
            .push(rule.id.clone());
        domains
            .entry(top_level_domain(&rule.category).to_string())
            .or_default()
            .push(rule.id.clone());
    }

    let recommended_packs = build_recommended_packs(&phases, &domains);
    DiagnosticPackPlan {
        schema: "sarakura-diagnostic-pack-plan".to_string(),
        schema_version: 1,
        platform: platform.as_str().to_string(),
        phases,
        domains,
        recommended_packs,
    }
}

// Accept any requested pack match; an empty filter or all accepts everything.
// Core requires MVP-1. Other names match the normalized phase, top-level domain
// or complete category, without looking up the recommended-pack list.
pub fn diagnostic_matches_pack(
    phase: Option<&str>,
    category: Option<&str>,
    packs: &[String],
) -> bool {
    if packs.is_empty() {
        return true;
    }
    packs.iter().any(|pack| {
        let p = normalize_pack_name(pack);
        if p == "all" {
            return true;
        }
        if p == "core" || p == "mvp1" || p == "mvp-1" {
            return phase
                .map(|x| x.eq_ignore_ascii_case("MVP-1"))
                .unwrap_or(false);
        }
        if let Some(phase) = phase {
            if normalize_pack_name(phase) == p {
                return true;
            }
        }
        if let Some(category) = category {
            let domain = normalize_pack_name(top_level_domain(category));
            if domain == p {
                return true;
            }
            let full = normalize_pack_name(category);
            if full == p {
                return true;
            }
        }
        false
    })
}

// Offer core only for the exact MVP-1 phase key, then known domain groups in
// the fixed order below. Group lookup is case-sensitive even though filtering
// normalizes requested names; rule IDs remain in their original catalog order.
fn build_recommended_packs(
    phases: &BTreeMap<String, Vec<String>>,
    domains: &BTreeMap<String, Vec<String>>,
) -> Vec<DiagnosticPack> {
    let mut packs = Vec::new();
    if let Some(rule_ids) = phases.get("MVP-1") {
        packs.push(DiagnosticPack {
            name: "core".to_string(),
            description: "MVP-1 only. Use this first for white-screen, timing, bank, and audio stopper diagnostics.".to_string(),
            rule_ids: rule_ids.clone(),
        });
    }
    for domain in [
        "PPU", "DMA", "OAM", "Audio", "Bank", "Mapper", "FDS", "VRC7", "Replay",
    ] {
        if let Some(rule_ids) = domains.get(domain) {
            packs.push(DiagnosticPack {
                name: normalize_pack_name(domain),
                description: format!("{} domain diagnostics.", domain),
                rule_ids: rule_ids.clone(),
            });
        }
    }
    packs
}

// Use the first slash-separated category component, including an empty one.
// No trimming or case normalization is performed when building group keys.
fn top_level_domain(category: &str) -> &str {
    category.split('/').next().unwrap_or(category)
}

// Lowercase ASCII and remove punctuation, whitespace and non-ASCII characters
// so phase/category spellings can be compared through one compact filter key.
fn normalize_pack_name(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagnosticRule, FixturePair};

    // Build a catalog fixture with configurable grouping fields and no repair metadata.
    fn rule(id: &str, phase: &str, category: &str) -> DiagnosticRule {
        DiagnosticRule {
            id: id.to_string(),
            event_type: format!("{}_EVENT", id),
            phase: phase.to_string(),
            severity: "warn".to_string(),
            category: category.to_string(),
            detection_condition: String::new(),
            required_metadata: Vec::new(),
            event_fields: Vec::new(),
            repair_target_candidates: Vec::new(),
            safe_patch_hint: String::new(),
            fixtures: FixturePair::default(),
            codex_tasks: Vec::new(),
        }
    }

    #[test]
    // Check phase and domain membership plus the presence of the recommended core pack.
    fn builds_phase_and_domain_packs() {
        let plan = build_pack_plan(
            Platform::Gb,
            &[
                rule("A", "MVP-1", "PPU/VRAM"),
                rule("B", "MVP-2", "Audio/APU"),
            ],
        );
        assert_eq!(plan.phases.get("MVP-1").unwrap(), &vec!["A".to_string()]);
        assert_eq!(plan.domains.get("Audio").unwrap(), &vec!["B".to_string()]);
        assert!(plan.recommended_packs.iter().any(|p| p.name == "core"));
    }

    #[test]
    // Verify core and audio selection, and rejection of an unrelated PPU filter.
    fn matches_core_and_domain_packs() {
        assert!(diagnostic_matches_pack(
            Some("MVP-1"),
            Some("PPU/VRAM"),
            &["core".to_string()]
        ));
        assert!(diagnostic_matches_pack(
            Some("MVP-2"),
            Some("Audio/APU"),
            &["audio".to_string()]
        ));
        assert!(!diagnostic_matches_pack(
            Some("MVP-2"),
            Some("Audio/APU"),
            &["ppu".to_string()]
        ));
    }
}

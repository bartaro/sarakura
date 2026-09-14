// Core analysis modules operate on supplied observations and metadata.
// Public re-exports below expose report/plan construction without emulator execution.
pub mod analyze;
pub mod automation_plan;
pub mod baseline;
pub mod catalog;
pub mod ci;
pub mod confidence;
pub mod coverage;
pub mod event_loader;
pub mod inspect;
pub mod metadata;
pub mod model;
pub mod normalize;
pub mod output;
pub mod packs;
pub mod redaction;
pub mod repair_plan;

// Convenience entry points for consumers; shared data structures are re-exported
// from model while specialized modules remain available by module path.
pub use analyze::{analyze, AnalyzeInput};
pub use automation_plan::{
    build_automation_plan, build_automation_plan_with_capabilities, render_automation_plan_markdown,
};
pub use baseline::{build_baseline_delta, render_baseline_delta_markdown, BaselineDeltaReport};
pub use catalog::load_catalog_from_str;
pub use ci::{build_ci_summary, CiSummary, CountItem};
pub use event_loader::load_events_jsonl;
pub use inspect::{
    build_retest_plan, inspect_events, inspect_metadata, validate_emitter_compatibility,
    validate_emitter_compatibility_with_options, write_retest_plan, EmitterCompatibilityOptions,
};
pub use metadata::load_metadata;
pub use model::*;
pub use normalize::{normalize_events, write_events_jsonl};
pub use output::{write_ai_diagnostics, write_diagnostic_summary, write_repair_prompt};
pub use redaction::redact_document;
pub use repair_plan::{build_repair_plan, render_repair_plan_markdown};

pub use coverage::build_catalog_coverage;
pub use packs::{build_pack_plan, diagnostic_matches_pack};

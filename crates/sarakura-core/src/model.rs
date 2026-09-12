use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Gb,
    Fc,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Gb => "gb",
            Platform::Fc => "fc",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticRule {
    pub id: String,
    pub event_type: String,
    pub phase: String,
    pub severity: String,
    pub category: String,
    pub detection_condition: String,
    pub required_metadata: Vec<String>,
    pub event_fields: Vec<String>,
    pub repair_target_candidates: Vec<String>,
    pub safe_patch_hint: String,
    pub fixtures: FixturePair,
    pub codex_tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FixturePair {
    pub red: Option<String>,
    pub green: Option<String>,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub schema: Option<String>,
    pub schema_version: Option<u32>,
    pub event_id: Option<String>,
    pub event_type: String,
    pub severity: Option<String>,
    pub frame: Option<u64>,
    pub scanline: Option<i64>,
    pub dot: Option<i64>,
    pub cpu_cycle: Option<u64>,
    pub pc: Option<String>,
    pub bank: Option<i64>,
    pub prg_bank: Option<i64>,
    pub addr: Option<String>,
    pub value: Option<String>,
    pub function_id_guess: Option<String>,
    pub symbol_hint: Option<String>,
    pub function_hint: Option<String>,
    pub op_id_guess: Option<String>,
    pub summary_key: Option<String>,
    pub count: Option<u64>,
    pub first_seen: Option<u64>,
    pub last_seen: Option<u64>,
    pub snapshot_ref: Option<String>,
    pub trace_window_ref: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl DiagnosticEvent {
    pub fn stable_key(&self) -> String {
        if let Some(key) = &self.summary_key {
            return key.clone();
        }
        format!(
            "{}|pc={}|bank={:?}|prg_bank={:?}|addr={}",
            self.event_type,
            self.pc.as_deref().unwrap_or("?"),
            self.bank,
            self.prg_bank,
            self.addr.as_deref().unwrap_or("?")
        )
    }

    pub fn normalized_count(&self) -> u64 {
        self.count.unwrap_or(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildMetadata {
    #[serde(flatten)]
    pub raw: BTreeMap<String, Value>,
}

impl BuildMetadata {
    pub fn schema(&self) -> Option<&str> {
        self.raw.get("schema").and_then(Value::as_str)
    }

    pub fn build_id(&self) -> Option<&str> {
        self.raw.get("build_id").and_then(Value::as_str)
    }

    pub fn rom_path(&self) -> Option<&str> {
        self.raw
            .get("rom")
            .and_then(|v| v.get("path"))
            .and_then(Value::as_str)
    }

    pub fn rom_hash(&self) -> Option<&str> {
        self.raw
            .get("rom")
            .and_then(|v| v.get("hash").or_else(|| v.get("sha256")))
            .and_then(Value::as_str)
    }

    pub fn target(&self) -> Option<&str> {
        self.raw.get("target").and_then(Value::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeOptions {
    pub platform: Platform,
    pub frames: u64,
    pub summary_limit: usize,
    pub event_type_filter: Vec<String>,
    pub phase_filter: Vec<String>,
    pub pack_filter: Vec<String>,
    pub min_severity: Option<String>,
    #[serde(default)]
    pub allow_project_labels: bool,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            platform: Platform::Gb,
            frames: 300,
            summary_limit: 200,
            event_type_filter: Vec::new(),
            phase_filter: Vec::new(),
            pack_filter: Vec::new(),
            min_severity: None,
            allow_project_labels: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDiagnosticsDocument {
    pub schema: String,
    pub schema_version: u32,
    pub producer: String,
    pub platform: String,
    pub build_id: Option<String>,
    pub rom: RomSummary,
    pub run: RunSummary,
    pub summary: DiagnosticSummary,
    pub diagnostics: Vec<AiDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub frames_requested: u64,
    pub diagnostic_summary_limit: usize,
    pub events_loaded: usize,
    pub events_aggregated: usize,
    pub diagnostics_before_filter: usize,
    pub catalog_rules_total: usize,
    pub event_type_filter: Vec<String>,
    pub phase_filter: Vec<String>,
    pub pack_filter: Vec<String>,
    pub min_severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RomSummary {
    pub path: Option<String>,
    pub target: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticSummary {
    pub frames_analyzed: u64,
    pub diagnostics_total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub unmapped_diagnostics: usize,
    pub event_types: BTreeMap<String, usize>,
    pub phases: BTreeMap<String, usize>,
    pub categories: BTreeMap<String, usize>,
    pub repair_targets: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDiagnostic {
    pub diagnostic_id: String,
    pub catalog_id: Option<String>,
    pub diagnostic_type: String,
    pub severity: String,
    pub phase: Option<String>,
    pub category: Option<String>,
    pub confidence: f32,
    pub source_mapping: Option<SourceMapping>,
    pub evidence_chain: Vec<EvidenceItem>,
    pub repair_target: RepairTarget,
    pub target_candidates: Vec<TargetCandidate>,
    pub safe_patch_hint: String,
    pub retest_condition: RetestCondition,
    pub event_count: u64,
    pub first_seen: Option<u64>,
    pub last_seen: Option<u64>,
    pub snapshot_ref: Option<String>,
    pub trace_window_ref: Option<String>,
    pub sample_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMapping {
    pub source_file: Option<String>,
    pub source_line: Option<u64>,
    pub function_id: Option<String>,
    pub function_name: Option<String>,
    pub symbol_name: Option<String>,
    pub op_id: Option<String>,
    pub pc: Option<String>,
    pub bank: Option<i64>,
    pub prg_bank: Option<i64>,
    pub mapping_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prg_bank: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairTarget {
    pub target_type: String,
    pub primary_file: Option<String>,
    pub primary_line: Option<u64>,
    pub function_id: Option<String>,
    pub function_name: Option<String>,
    pub op_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetCandidate {
    pub target_type: String,
    pub rank: u32,
    pub reason: String,
    pub metadata_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestCondition {
    pub frames: u64,
    pub expect_absent: Vec<String>,
    pub expect_not_worse: Vec<String>,
    pub required_outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameRange {
    pub first: Option<u64>,
    pub last: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventInspectionSummary {
    pub events_loaded: usize,
    pub effective_count: u64,
    pub event_types: BTreeMap<String, u64>,
    pub severities: BTreeMap<String, u64>,
    pub frames: FrameRange,
    pub event_ids_present: usize,
    pub snapshot_refs: usize,
    pub trace_window_refs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataInspectionSummary {
    pub schema: Option<String>,
    pub build_id: Option<String>,
    pub target: Option<String>,
    pub rom_path: Option<String>,
    pub rom_hash: Option<String>,
    pub array_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestPlan {
    pub schema: String,
    pub schema_version: u32,
    pub platform: String,
    pub build_id: Option<String>,
    pub frames: u64,
    pub expect_absent: Vec<String>,
    pub expect_not_worse: Vec<String>,
    pub required_outputs: Vec<String>,
    pub diagnostics: Vec<RetestDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestDiagnostic {
    pub diagnostic_id: String,
    pub diagnostic_type: String,
    pub severity: String,
    pub catalog_id: Option<String>,
    pub repair_target: RepairTarget,
    pub frames: u64,
    pub expect_absent: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogCoverageReport {
    pub schema: String,
    pub schema_version: u32,
    pub platform: String,
    pub catalog_rules_total: usize,
    pub observed_catalog_rules: usize,
    pub missing_catalog_rules: usize,
    pub unknown_event_types: Vec<String>,
    pub unknown_event_suggestions: Vec<UnknownEventSuggestion>,
    pub event_types_seen: BTreeMap<String, u64>,
    pub rules: Vec<CatalogCoverageRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnknownEventSuggestion {
    pub event_type: String,
    pub candidate_catalog_ids: Vec<String>,
    pub candidate_event_types: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogCoverageRow {
    pub id: String,
    pub event_type: String,
    pub phase: String,
    pub severity: String,
    pub category: String,
    pub observed: bool,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticPackPlan {
    pub schema: String,
    pub schema_version: u32,
    pub platform: String,
    pub phases: BTreeMap<String, Vec<String>>,
    pub domains: BTreeMap<String, Vec<String>>,
    pub recommended_packs: Vec<DiagnosticPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticPack {
    pub name: String,
    pub description: String,
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepairPlan {
    pub schema: String,
    pub schema_version: u32,
    pub platform: String,
    pub build_id: Option<String>,
    pub generated_from: String,
    pub summary: RepairPlanSummary,
    pub steps: Vec<RepairPlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepairPlanSummary {
    pub total_steps: usize,
    pub error_steps: usize,
    pub warning_steps: usize,
    pub info_steps: usize,
    pub diagnostics_covered: usize,
    pub target_groups: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPlanStep {
    pub step_id: String,
    pub priority: u32,
    pub severity: String,
    pub repair_target_type: String,
    pub primary_file: Option<String>,
    pub primary_line: Option<u64>,
    pub function_id: Option<String>,
    pub function_name: Option<String>,
    pub op_id: Option<String>,
    pub diagnostic_ids: Vec<String>,
    pub diagnostic_types: Vec<String>,
    pub catalog_ids: Vec<String>,
    pub event_count: u64,
    pub max_confidence: f32,
    pub safe_patch_hints: Vec<String>,
    pub retest_condition: RetestCondition,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutomationPlan {
    pub schema: String,
    pub schema_version: u32,
    pub platform: String,
    pub build_id: Option<String>,
    pub generated_from: String,
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ToolCapabilities>,
    pub summary: AutomationPlanSummary,
    pub commands: Vec<AutomationCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutomationPlanSummary {
    pub total_commands: usize,
    pub reproduce_commands: usize,
    pub inspect_commands: usize,
    pub retest_commands: usize,
    pub diagnostic_commands: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationCommand {
    pub command_id: String,
    pub stage: String,
    pub purpose: String,
    pub tool: String,
    pub cli: String,
    pub diagnostic_id: Option<String>,
    pub diagnostic_type: Option<String>,
    pub catalog_id: Option<String>,
    pub severity: Option<String>,
    pub event_type: Option<String>,
    pub pc: Option<String>,
    pub frame: Option<u64>,
    pub snapshot_out: Option<String>,
    pub trace_out: Option<String>,
    pub json_out: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCapabilities {
    pub schema: Option<String>,
    pub schema_version: Option<u32>,
    pub tool: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReproBundleInspection {
    pub schema: String,
    pub schema_version: u32,
    pub path: String,
    pub manifest_present: bool,
    pub manifest: Option<Value>,
    pub entries: Vec<ReproBundleEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReproBundleEntry {
    pub name: String,
    pub size: u64,
}

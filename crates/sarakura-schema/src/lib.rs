use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct SchemaEntry {
    pub file_name: String,
    pub schema: Value,
}

pub fn validate_ai_diagnostics_file(path: impl AsRef<Path>) -> Result<()> {
    validate_ai_diagnostics_file_with_options(path, false)
}

pub fn validate_ai_diagnostics_file_with_options(
    path: impl AsRef<Path>,
    strict: bool,
) -> Result<()> {
    let path = path.as_ref();
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    validate_ai_diagnostics_value_with_options(&json, strict)
}

pub fn validate_ai_diagnostics_value(json: &Value) -> Result<()> {
    validate_ai_diagnostics_value_with_options(json, false)
}

pub fn validate_ai_diagnostics_value_with_options(json: &Value, strict: bool) -> Result<()> {
    let schema = json
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !schema.contains("sarakura") || !schema.contains("ai-diagnostics") {
        bail!("schema must be sarakura-*-ai-diagnostics, got {:?}", schema);
    }
    required_u64(json, "schema_version")?;
    required_object(json, "rom")?;
    required_object(json, "run")?;
    required_object(json, "summary")?;
    if strict {
        required_str(json, "platform")?;
        if json.get("build_id").is_none() {
            bail!("build_id is required in strict mode");
        }
    }
    let diagnostics = json
        .get("diagnostics")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("diagnostics array is required"))?;

    let mut seen_ids = BTreeSet::new();
    for (idx, diag) in diagnostics.iter().enumerate() {
        let diagnostic_id = required_str_at(diag, "diagnostic_id", idx)?;
        if strict && !seen_ids.insert(diagnostic_id.to_string()) {
            bail!("diagnostics[{idx}].diagnostic_id duplicates {diagnostic_id:?}");
        }
        let diagnostic_type = required_str_at(diag, "diagnostic_type", idx)?;
        if strict && diagnostic_type.trim().is_empty() {
            bail!("diagnostics[{idx}].diagnostic_type must not be empty");
        }
        let severity = required_str_at(diag, "severity", idx)?;
        if !matches!(severity, "error" | "warn" | "info") {
            bail!("diagnostics[{idx}].severity must be error|warn|info, got {severity:?}");
        }
        let confidence = required_number_at(diag, "confidence", idx)?;
        if strict && !(0.0..=1.0).contains(&confidence) {
            bail!("diagnostics[{idx}].confidence must be between 0.0 and 1.0");
        }
        let repair_target = required_object_at(diag, "repair_target", idx)?;
        required_array_at(diag, "evidence_chain", idx)?;
        let retest_condition = required_object_at(diag, "retest_condition", idx)?;
        if strict {
            let target_type = repair_target
                .get("target_type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if target_type.trim().is_empty() {
                bail!("diagnostics[{idx}].repair_target.target_type must not be empty");
            }
            let expect_absent = retest_condition
                .get("expect_absent")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "diagnostics[{idx}].retest_condition.expect_absent must be an array"
                    )
                })?;
            if expect_absent.is_empty() {
                bail!("diagnostics[{idx}].retest_condition.expect_absent must not be empty");
            }
        }
    }
    Ok(())
}

pub fn schema_entries() -> Vec<SchemaEntry> {
    vec![
        schema_entry(
            "sarakura_ai_diagnostics.schema.json",
            "sarakura-*-ai-diagnostics",
        ),
        schema_entry(
            "sarakura_diagnostic_summary.schema.json",
            "sarakura-diagnostic-summary",
        ),
        schema_entry("sarakura_retest_plan.schema.json", "sarakura-retest-plan"),
        schema_entry("sarakura_repair_plan.schema.json", "sarakura-repair-plan"),
        schema_entry(
            "sarakura_automation_plan.schema.json",
            "sarakura-automation-plan",
        ),
        schema_entry(
            "sarakura_baseline_delta.schema.json",
            "sarakura-baseline-delta",
        ),
        schema_entry("sarakura_ci_summary.schema.json", "sarakura-ci-summary"),
        schema_entry(
            "sarakura_catalog_coverage.schema.json",
            "sarakura-catalog-coverage",
        ),
        schema_entry(
            "sarakura_diagnostic_pack_plan.schema.json",
            "sarakura-diagnostic-pack-plan",
        ),
        schema_entry(
            "sarakura_repro_bundle_manifest.schema.json",
            "sarakura-repro-bundle-manifest",
        ),
        schema_entry(
            "sarakura_diagnostic_event.schema.json",
            "kokura-or-kurosaki-diagnostic-event",
        ),
        schema_entry(
            "sarakura_build_metadata.schema.json",
            "kitaq-build-metadata",
        ),
        schema_entry(
            "sarakura_normalized_events.schema.json",
            "sarakura-normalized-events",
        ),
    ]
}

pub fn write_schema_bundle(out_dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;
    let mut written = Vec::new();
    for entry in schema_entries() {
        let path = out_dir.join(&entry.file_name);
        fs::write(&path, serde_json::to_string_pretty(&entry.schema)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
        written.push(path.display().to_string());
    }
    Ok(written)
}

fn schema_entry(file_name: &str, schema_name: &str) -> SchemaEntry {
    SchemaEntry {
        file_name: file_name.to_string(),
        schema: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": format!("https://kitaq.dev/sarakura/schemas/{file_name}"),
            "title": schema_name,
            "type": "object",
            "required": ["schema", "schema_version"],
            "properties": {
                "schema": { "type": "string" },
                "schema_version": { "type": "integer", "minimum": 1 }
            },
            "additionalProperties": true
        }),
    }
}

fn required_object<'a>(json: &'a Value, key: &str) -> Result<&'a serde_json::Map<String, Value>> {
    json.get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("{key} object is required"))
}

fn required_u64(json: &Value, key: &str) -> Result<u64> {
    json.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("{key} is required and must be u64"))
}

fn required_str<'a>(json: &'a Value, key: &str) -> Result<&'a str> {
    json.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{key} is required and must be string"))
}

fn required_str_at<'a>(json: &'a Value, key: &str, idx: usize) -> Result<&'a str> {
    json.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("diagnostics[{idx}].{key} is required and must be string"))
}

fn required_number_at(json: &Value, key: &str, idx: usize) -> Result<f64> {
    json.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("diagnostics[{idx}].{key} is required and must be number"))
}

fn required_object_at<'a>(
    json: &'a Value,
    key: &str,
    idx: usize,
) -> Result<&'a serde_json::Map<String, Value>> {
    json.get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("diagnostics[{idx}].{key} is required and must be object"))
}

fn required_array_at(json: &Value, key: &str, idx: usize) -> Result<()> {
    if json.get(key).and_then(Value::as_array).is_none() {
        bail!("diagnostics[{idx}].{key} is required and must be array");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_bundle_contains_v1_contract_files() {
        let entries = schema_entries();
        assert!(entries
            .iter()
            .any(|entry| entry.file_name == "sarakura_repro_bundle_manifest.schema.json"));
        assert!(entries
            .iter()
            .any(|entry| entry.file_name == "sarakura_diagnostic_summary.schema.json"));
    }

    #[test]
    fn strict_validation_rejects_empty_retest_condition() {
        let value = json!({
            "schema": "sarakura-gb-ai-diagnostics",
            "schema_version": 1,
            "platform": "gb",
            "build_id": "b1",
            "rom": {},
            "run": {},
            "summary": {},
            "diagnostics": [{
                "diagnostic_id": "d1",
                "diagnostic_type": "VRAM_WRITE_OUTSIDE_SAFE_PERIOD",
                "severity": "error",
                "confidence": 0.8,
                "repair_target": {"target_type": "vblank_queue"},
                "evidence_chain": [],
                "retest_condition": {"expect_absent": []}
            }]
        });
        assert!(validate_ai_diagnostics_value_with_options(&value, true).is_err());
    }
}

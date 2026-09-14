use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Read;
use std::path::PathBuf;
use tempfile::TempDir;

// Locate bundled fixtures relative to this crate, independent of the test
// process working directory.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
// Run the real CLI for one synthetic event per built-in rule plus an unknown
// event. Check output and ZIP text for the selected Japanese/CJK ranges and
// the English HTML language tag; this is not a natural-language quality test.
fn all_builtin_diagnostics_generate_english_reports() {
    let root = workspace_root();
    for platform in ["gb", "fc"] {
        let work = TempDir::new().unwrap();
        let catalog_path = root.join(format!(
            "crates/sarakura-{platform}/src/{platform}_catalog.json"
        ));
        let catalog_text = std::fs::read_to_string(catalog_path).unwrap();
        let rules: Vec<serde_json::Value> = serde_json::from_str(&catalog_text).unwrap();
        // Exercise every built-in hint, candidate reason, and evidence path, plus
        // the unmapped-event fallback, through the real CLI and bundle writer.
        let mut events: Vec<String> = rules
            .iter()
            .enumerate()
            .map(|(i, rule)| {
                serde_json::json!({
                    "event_type": rule["event_type"],
                    "event_id": format!("sample_{i}"),
                    "frame": i,
                    "pc": "0x0100"
                })
                .to_string()
            })
            .collect();
        events.push(serde_json::json!({"event_type":"UNREGISTERED_SAMPLE_EVENT"}).to_string());
        let events_path = work.path().join("events.jsonl");
        std::fs::write(&events_path, events.join("\n")).unwrap();
        let metadata_path = work.path().join("build.json");
        std::fs::write(&metadata_path, "{}").unwrap();
        let output = work.path().join("output");
        Command::cargo_bin("sarakura")
            .unwrap()
            .args([platform, "analyze", "--fail-on", "never"])
            .arg("--metadata")
            .arg(metadata_path)
            .arg("--events")
            .arg(events_path)
            .arg("--out")
            .arg(&output)
            .assert()
            .success();
        let is_japanese = |c: char| matches!(c, '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}');
        assert!(!catalog_text.chars().any(is_japanese));
        for name in [
            "ai_diagnostics.json",
            "repair_prompt.md",
            "repair_plan.json",
            "repair_plan.md",
            "automation_plan.json",
            "automation_plan.md",
            "report.html",
        ] {
            let text = std::fs::read_to_string(output.join(name)).unwrap();
            assert!(
                !text.chars().any(is_japanese),
                "{platform}/{name} contains Japanese built-in text"
            );
            if name == "report.html" {
                assert!(text.contains("lang=\"en\""));
            }
        }
        let diagnostics: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join("ai_diagnostics.json")).unwrap())
                .unwrap();
        assert_eq!(
            diagnostics["diagnostics"].as_array().unwrap().len(),
            rules.len() + 1
        );
        let mut bundle =
            zip::ZipArchive::new(std::fs::File::open(output.join("repro_bundle.zip")).unwrap())
                .unwrap();
        for index in 0..bundle.len() {
            let mut file = bundle.by_index(index).unwrap();
            let mut text = String::new();
            file.read_to_string(&mut text).unwrap();
            assert!(
                !text.chars().any(is_japanese),
                "Japanese in bundled {}",
                file.name()
            );
        }
    }
}

#[test]
// Check that an absent validation input fails with a file-read diagnostic.
fn validate_rejects_missing_file() {
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.args(["validate", "does-not-exist.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
// Exercise CLI schema export and check two required files plus its status line.
fn schema_export_writes_v1_contract_files() {
    let out = TempDir::new().unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("schema")
        .arg("export")
        .arg("--out")
        .arg(out.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("schema files"));
    assert!(out
        .path()
        .join("sarakura_repro_bundle_manifest.schema.json")
        .exists());
    assert!(out
        .path()
        .join("sarakura_diagnostic_summary.schema.json")
        .exists());
}

#[test]
// Exercise GB report generation, default identity redaction and omission of
// original inputs from the ZIP; then validate and inspect the emitted artifacts.
fn gb_analyze_writes_expected_files() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote SARAKURA diagnostics"));

    assert!(out.path().join("ai_diagnostics.json").exists());
    assert!(out.path().join("diagnostic_summary.json").exists());
    assert!(out.path().join("report.html").exists());
    assert!(out.path().join("repro_bundle.zip").exists());
    assert!(out.path().join("repair_plan.json").exists());
    assert!(out.path().join("repair_plan.md").exists());

    let diagnostics: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.path().join("ai_diagnostics.json")).unwrap())
            .unwrap();
    assert_eq!(diagnostics["build_id"], "project_0001");
    assert_eq!(diagnostics["rom"]["path"], "input_0001");
    assert!(diagnostics["rom"]["hash"].is_null());

    let bundle_file = std::fs::File::open(out.path().join("repro_bundle.zip")).unwrap();
    let mut bundle = zip::ZipArchive::new(bundle_file).unwrap();
    assert!((0..bundle.len()).all(|index| {
        !bundle
            .by_index(index)
            .unwrap()
            .name()
            .starts_with("inputs/")
    }));
    let mut manifest_text = String::new();
    bundle
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest_text)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest["source_inputs_omitted"], true);
    assert_eq!(manifest["inputs"], serde_json::json!([]));

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("validate")
        .arg(out.path().join("ai_diagnostics.json"))
        .arg("--strict")
        .assert()
        .success();

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("inspect-repro")
        .arg(out.path().join("repro_bundle.zip"))
        .assert()
        .success()
        .stdout(predicate::str::contains("sarakura-repro-bundle-inspection"));
}

#[test]
// Check that FC analysis still writes diagnostics when both optional HTML
// and ZIP outputs are disabled.
fn fc_analyze_supports_no_report_flags() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.arg("fc")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/fc/kitaqfc_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/fc/kurosaki_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .arg("--no-report")
        .arg("--no-repro-bundle")
        .assert()
        .success();

    assert!(out.path().join("ai_diagnostics.json").exists());
    assert!(!out.path().join("report.html").exists());
    assert!(!out.path().join("repro_bundle.zip").exists());
}

#[test]
// Check that the GB catalog command exposes a known rule in its default table.
fn catalog_command_lists_gb_rules() {
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.args(["catalog", "gb"])
        .assert()
        .success()
        .stdout(predicate::str::contains("GBD001"));
}

#[test]
// Smoke-test parsing and execution of phase, severity and fail-on options.
// This assertion checks output presence, not the exact retained diagnostic set.
fn gb_analyze_supports_filters() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .arg("--phase")
        .arg("MVP-1")
        .arg("--min-severity")
        .arg("warn")
        .arg("--fail-on")
        .arg("never")
        .assert()
        .success();

    assert!(out.path().join("ai_diagnostics.json").exists());
}

#[test]
// Exercise event inspection and check its compact count output.
fn inspect_events_reports_counts() {
    let root = workspace_root();
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.arg("inspect-events")
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .assert()
        .success()
        .stdout(predicate::str::contains("events="));
}

#[test]
// Check that ordinary analysis emits a retest-plan file alongside diagnostics.
fn analyze_writes_retest_plan() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();

    assert!(out.path().join("retest_plan.json").exists());
}

#[test]
// Generate diagnostics, pass their directory to retest-plan, and check the
// JSON schema marker printed when no output path is supplied.
fn retest_plan_command_prints_json() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("retest-plan")
        .arg("--diagnostics")
        .arg(out.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("sarakura-retest-plan"));
}

#[test]
// Check the emitter-check summary path using bundled metadata and events.
// This smoke test does not require zero compatibility warnings.
fn emitter_check_reports_compatibility() {
    let root = workspace_root();
    let mut cmd = Command::cargo_bin("sarakura").unwrap();
    cmd.arg("emitter-check")
        .arg("gb")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .assert()
        .success()
        .stdout(predicate::str::contains("emitter compatibility warnings="));
}

#[test]
// Exercise normalization and confirm its output file and status message.
// Detailed aggregation semantics are covered by core tests.
fn normalize_events_writes_jsonl() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let out_file = out.path().join("normalized.jsonl");
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("normalize-events")
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(&out_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("normalized events"));
    assert!(out_file.exists());
}

#[test]
// Generate diagnostics and check CI-summary JSON under the non-failing policy.
fn ci_summary_reports_status() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("ci-summary")
        .arg("--diagnostics")
        .arg(out.path())
        .arg("--fail-on")
        .arg("never")
        .assert()
        .success()
        .stdout(predicate::str::contains("sarakura-ci-summary"));
}

#[test]
// Run GB coverage on fixture events and check its JSON file and platform summary.
fn coverage_reports_catalog_observation() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let out_file = out.path().join("coverage.json");
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("coverage")
        .arg("gb")
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(&out_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("coverage platform=gb"));
    assert!(out_file.exists());
}

#[test]
// Check the default pack-plan Markdown includes recommendations and the core pack.
fn pack_plan_lists_recommended_packs() {
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("pack-plan")
        .arg("gb")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recommended packs"))
        .stdout(predicate::str::contains("core"));
}

#[test]
// Exercise the core-pack filter through the CLI and check output creation.
// The exact filtered diagnostic set is not asserted by this smoke test.
fn analyze_supports_diagnostic_pack_filter() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .arg("--diagnostic-pack")
        .arg("core")
        .assert()
        .success();
    assert!(out.path().join("ai_diagnostics.json").exists());
}

#[test]
// Generate a diagnostics directory, then verify repair-plan accepts output
// directories and writes both requested formats.
fn repair_plan_command_writes_json_and_markdown() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let repair_out = TempDir::new().unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("repair-plan")
        .arg("--diagnostics")
        .arg(out.path())
        .arg("--out")
        .arg(repair_out.path())
        .arg("--markdown")
        .arg(repair_out.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("repair plan steps="));

    assert!(repair_out.path().join("repair_plan.json").exists());
    assert!(repair_out.path().join("repair_plan.md").exists());
}

#[test]
// Check that analysis emits both JSON and Markdown automation proposals.
fn analyze_writes_automation_plan() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("gb")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();

    assert!(out.path().join("automation_plan.json").exists());
    assert!(out.path().join("automation_plan.md").exists());
}

#[test]
// Generate FC diagnostics and check that an explicit KUROSAKI tool hint
// produces both requested plan files without executing their commands.
fn automation_plan_command_writes_json_and_markdown() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let automation_out = TempDir::new().unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("fc")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/fc/kitaqfc_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/fc/kurosaki_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("automation-plan")
        .arg("--diagnostics")
        .arg(out.path())
        .arg("--tool")
        .arg("kurosaki")
        .arg("--out")
        .arg(automation_out.path())
        .arg("--markdown")
        .arg(automation_out.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("automation plan commands="));

    assert!(automation_out.path().join("automation_plan.json").exists());
    assert!(automation_out.path().join("automation_plan.md").exists());
}

#[test]
// Supply a restricted KUROSAKI capability fixture and check omission of the
// diagnostic-break option while retaining JSON output in the generated plan.
fn automation_plan_honors_capability_file() {
    let root = workspace_root();
    let out = TempDir::new().unwrap();
    let automation_out = TempDir::new().unwrap();
    let capabilities_path = automation_out.path().join("capabilities.json");
    std::fs::write(
        &capabilities_path,
        r#"{"tool":"kurosaki","commands":["run"],"options":["--frames","--json"]}"#,
    )
    .unwrap();
    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("fc")
        .arg("analyze")
        .arg("--metadata")
        .arg(root.join("examples/fc/kitaqfc_build_metadata.json"))
        .arg("--events")
        .arg(root.join("examples/fc/kurosaki_diagnostic_events.jsonl"))
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("automation-plan")
        .arg("--diagnostics")
        .arg(out.path())
        .arg("--tool")
        .arg("kurosaki")
        .arg("--capabilities")
        .arg(&capabilities_path)
        .arg("--out")
        .arg(automation_out.path())
        .assert()
        .success();

    let plan = std::fs::read_to_string(automation_out.path().join("automation_plan.json")).unwrap();
    assert!(!plan.contains("--break-on-diagnostic"));
    assert!(plan.contains("--json"));
}

#[test]
// Compare two reports generated from identical inputs, enforce error policies,
// and check passing status plus both output formats. No regression is injected.
fn baseline_delta_command_writes_json_and_markdown() {
    let root = workspace_root();
    let baseline = TempDir::new().unwrap();
    let current = TempDir::new().unwrap();
    let delta_out = TempDir::new().unwrap();
    for out in [baseline.path(), current.path()] {
        Command::cargo_bin("sarakura")
            .unwrap()
            .arg("gb")
            .arg("analyze")
            .arg("--metadata")
            .arg(root.join("examples/gb/kitaqgb_build_metadata.json"))
            .arg("--events")
            .arg(root.join("examples/gb/kokura_diagnostic_events.jsonl"))
            .arg("--out")
            .arg(out)
            .assert()
            .success();
    }

    Command::cargo_bin("sarakura")
        .unwrap()
        .arg("baseline-delta")
        .arg("--baseline")
        .arg(baseline.path())
        .arg("--current")
        .arg(current.path())
        .arg("--out")
        .arg(delta_out.path())
        .arg("--markdown")
        .arg(delta_out.path())
        .arg("--fail-on-new")
        .arg("error")
        .arg("--fail-on-regression")
        .arg("error")
        .arg("--enforce")
        .assert()
        .success()
        .stdout(predicate::str::contains("baseline delta status=passed"));

    assert!(delta_out.path().join("baseline_delta.json").exists());
    assert!(delta_out.path().join("baseline_delta.md").exists());
}

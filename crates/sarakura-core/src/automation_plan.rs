use crate::model::{
    AiDiagnostic, AiDiagnosticsDocument, AutomationCommand, AutomationPlan, AutomationPlanSummary,
    ToolCapabilities,
};
use std::collections::BTreeSet;

// Build a command proposal using the shared default capability table. No tool
// is probed and no command is executed by this convenience entry point.
pub fn build_automation_plan(
    doc: &AiDiagnosticsDocument,
    tool_hint: Option<&str>,
) -> AutomationPlan {
    build_automation_plan_with_capabilities(doc, tool_hint, None)
}

// Generate reproduce/inspect pairs in diagnostic order, then one retest per
// sorted unique event type. Use supplied capabilities verbatim or assumed
// defaults; they are not verified against an installed executable.
pub fn build_automation_plan_with_capabilities(
    doc: &AiDiagnosticsDocument,
    tool_hint: Option<&str>,
    capabilities: Option<&ToolCapabilities>,
) -> AutomationPlan {
    let tool = resolve_tool(&doc.platform, tool_hint);
    let resolved_capabilities = capabilities
        .cloned()
        .or_else(|| Some(default_capabilities(&tool)));
    let mut commands = Vec::new();

    for diag in &doc.diagnostics {
        commands.push(build_reproduce_command(
            &tool,
            resolved_capabilities.as_ref(),
            doc,
            diag,
            commands.len() + 1,
        ));
        commands.push(build_inspect_command(
            &tool,
            resolved_capabilities.as_ref(),
            doc,
            diag,
            commands.len() + 1,
        ));
    }

    for event_type in unique_event_types(doc) {
        commands.push(build_retest_command(
            &tool,
            resolved_capabilities.as_ref(),
            doc,
            &event_type,
            commands.len() + 1,
        ));
    }

    let mut summary = AutomationPlanSummary::default();
    summary.total_commands = commands.len();
    // This summary field counts source diagnostics, not the two command records
    // generated for each diagnostic; stage totals count actual command records.
    summary.diagnostic_commands = doc.diagnostics.len();
    for cmd in &commands {
        match cmd.stage.as_str() {
            "reproduce" => summary.reproduce_commands += 1,
            "inspect" => summary.inspect_commands += 1,
            "retest" => summary.retest_commands += 1,
            _ => {}
        }
    }

    AutomationPlan {
        schema: "sarakura-automation-plan".to_string(),
        schema_version: 1,
        platform: doc.platform.clone(),
        build_id: doc.build_id.clone(),
        generated_from: "ai_diagnostics.json".to_string(),
        tool,
        capabilities: resolved_capabilities,
        summary,
        commands,
    }
}

// Render English guidance and the proposed CLI strings in Bash-marked fences.
// This does not select a host shell, execute commands or validate the embedded
// paths/options; labels and command strings are interpolated as provided.
pub fn render_automation_plan_markdown(plan: &AutomationPlan) -> String {
    let mut out = String::new();
    out.push_str("# SARAKURA automation plan\n\n");
    out.push_str(&format!("- Platform: `{}`\n", plan.platform));
    out.push_str(&format!("- Tool: `{}`\n", plan.tool));
    out.push_str(&format!(
        "- Build ID: `{}`\n",
        plan.build_id.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- Commands: `{}`\n\n",
        plan.summary.total_commands
    ));
    out.push_str("## Purpose\n\n");
    out.push_str(
        "This file turns SARAKURA diagnostics into concrete emulator automation commands.\n",
    );
    out.push_str(
        "Use it with KOKURA/KUROSAKI automation APIs, Python bindings, or CLI wrappers.\n\n",
    );

    for cmd in &plan.commands {
        out.push_str(&format!("## {} `{}`\n\n", cmd.command_id, cmd.stage));
        out.push_str(&format!("- Purpose: {}\n", cmd.purpose));
        if let Some(diag) = &cmd.diagnostic_id {
            out.push_str(&format!("- Diagnostic: `{}`\n", diag));
        }
        if let Some(event_type) = &cmd.event_type {
            out.push_str(&format!("- Event type: `{}`\n", event_type));
        }
        if let Some(pc) = &cmd.pc {
            out.push_str(&format!("- PC: `{}`\n", pc));
        }
        out.push_str("\n```bash\n");
        out.push_str(&cmd.cli);
        out.push_str("\n```\n\n");
    }
    out
}

// Propose a diagnostic-triggered run using this diagnostic's retest budget.
// Output fields describe requested artifacts even when capability filtering
// omits the corresponding CLI option; the paths are not created here.
fn build_reproduce_command(
    tool: &str,
    capabilities: Option<&ToolCapabilities>,
    doc: &AiDiagnosticsDocument,
    diag: &AiDiagnostic,
    index: usize,
) -> AutomationCommand {
    let command_id = format!("auto_{:04}", index);
    let rom = doc.rom.path.as_deref().unwrap_or("<rom>");
    let event_type = diag.diagnostic_type.clone();
    let json_out = format!("automation/{}_hit.json", diag.diagnostic_id);
    let snapshot_out = format!("automation/{}_snapshot.json", diag.diagnostic_id);
    let trace_out = format!("automation/{}_trace.jsonl", diag.diagnostic_id);
    let cli = build_run_cli(
        tool,
        capabilities,
        rom,
        diag.retest_condition.frames,
        Some(&event_type),
        Some(&json_out),
        Some(&snapshot_out),
        Some(&trace_out),
    );

    AutomationCommand {
        command_id,
        stage: "reproduce".to_string(),
        purpose: "Break at the first matching diagnostic and capture snapshot/trace evidence."
            .to_string(),
        tool: tool.to_string(),
        cli,
        diagnostic_id: Some(diag.diagnostic_id.clone()),
        diagnostic_type: Some(diag.diagnostic_type.clone()),
        catalog_id: diag.catalog_id.clone(),
        severity: Some(diag.severity.clone()),
        event_type: Some(event_type),
        pc: diag.source_mapping.as_ref().and_then(|m| m.pc.clone()),
        frame: diag.first_seen.or(diag.last_seen),
        snapshot_out: Some(snapshot_out),
        trace_out: Some(trace_out),
        json_out: Some(json_out),
    }
}

// Prefer auto-run-until when advertised, targeting PC before diagnostic type.
// Otherwise use a run breakpoint when PC is known or the capability-filtered
// run builder. Direct formatting branches do not quote fields or filter options.
fn build_inspect_command(
    tool: &str,
    capabilities: Option<&ToolCapabilities>,
    doc: &AiDiagnosticsDocument,
    diag: &AiDiagnostic,
    index: usize,
) -> AutomationCommand {
    let command_id = format!("auto_{:04}", index);
    let rom = doc.rom.path.as_deref().unwrap_or("<rom>");
    let json_out = format!("automation/{}_state.json", diag.diagnostic_id);
    let cli = if supports_command(capabilities, "auto-run-until") {
        if let Some(pc) = diag.source_mapping.as_ref().and_then(|m| m.pc.clone()) {
            format!(
                "{} auto-run-until {} --pc {} --json {}",
                tool, rom, pc, json_out
            )
        } else {
            format!(
                "{} auto-run-until {} --diagnostic {} --max-frames {} --json {}",
                tool, rom, diag.diagnostic_type, diag.retest_condition.frames, json_out
            )
        }
    } else if let Some(pc) = diag.source_mapping.as_ref().and_then(|m| m.pc.clone()) {
        format!(
            "{} run {} --frames {} --breakpoint {} --json {}",
            tool, rom, diag.retest_condition.frames, pc, json_out
        )
    } else {
        build_run_cli(
            tool,
            capabilities,
            rom,
            diag.retest_condition.frames,
            Some(&diag.diagnostic_type),
            Some(&json_out),
            None,
            None,
        )
    };

    AutomationCommand {
        command_id,
        stage: "inspect".to_string(),
        purpose: "Re-enter the suspect source/PC context for local memory and register inspection."
            .to_string(),
        tool: tool.to_string(),
        cli,
        diagnostic_id: Some(diag.diagnostic_id.clone()),
        diagnostic_type: Some(diag.diagnostic_type.clone()),
        catalog_id: diag.catalog_id.clone(),
        severity: Some(diag.severity.clone()),
        event_type: Some(diag.diagnostic_type.clone()),
        pc: diag.source_mapping.as_ref().and_then(|m| m.pc.clone()),
        frame: diag.first_seen.or(diag.last_seen),
        snapshot_out: None,
        trace_out: None,
        json_out: Some(json_out),
    }
}

// Propose a rerun and subsequent SARAKURA analysis for one event type. This
// uses the original run frame budget, not the largest per-diagnostic budget.
// The metadata placeholder and shared event-file path must be supplied/checked
// by the consumer; an error-only gate is not proof of absence of every severity.
fn build_retest_command(
    tool: &str,
    capabilities: Option<&ToolCapabilities>,
    doc: &AiDiagnosticsDocument,
    event_type: &str,
    index: usize,
) -> AutomationCommand {
    let command_id = format!("auto_{:04}", index);
    let rom = doc.rom.path.as_deref().unwrap_or("<rom>");
    let json_out = format!("automation/retest_{}.json", sanitize(event_type));
    let run_cli = build_run_cli(
        tool,
        capabilities,
        rom,
        doc.run.frames_requested,
        None,
        Some(&json_out),
        None,
        None,
    );
    let cli = format!(
        "{} && sarakura {} analyze --metadata <build_metadata.json> --events automation/retest_diagnostic_events.jsonl --out automation/sarakura_retest --diagnostic-rule {} --fail-on error",
        run_cli, doc.platform, event_type
    );

    AutomationCommand {
        command_id,
        stage: "retest".to_string(),
        purpose: "Run the ROM again and confirm the target diagnostic type is absent.".to_string(),
        tool: tool.to_string(),
        cli,
        diagnostic_id: None,
        diagnostic_type: Some(event_type.to_string()),
        catalog_id: None,
        severity: None,
        event_type: Some(event_type.to_string()),
        pc: None,
        frame: None,
        snapshot_out: None,
        trace_out: None,
        json_out: Some(json_out),
    }
}

// Assemble a run proposal: run and --frames are unconditional; optional flags
// are emitted only when advertised. All runs share one diagnostics output path.
// Snapshot-on-diagnostic uses a fixed directory instead of the requested file.
fn build_run_cli(
    tool: &str,
    capabilities: Option<&ToolCapabilities>,
    rom: &str,
    frames: u64,
    break_on_diagnostic: Option<&str>,
    json_out: Option<&str>,
    snapshot_out: Option<&str>,
    trace_out: Option<&str>,
) -> String {
    let mut parts = vec![
        tool.to_string(),
        "run".to_string(),
        shell_arg(rom),
        "--frames".to_string(),
        frames.to_string(),
    ];
    if supports_option(capabilities, "--emit-diagnostics") {
        parts.push("--emit-diagnostics".to_string());
        parts.push("automation/retest_diagnostic_events.jsonl".to_string());
    } else if supports_option(capabilities, "--diagnostics-jsonl") {
        parts.push("--diagnostics-jsonl".to_string());
        parts.push("automation/retest_diagnostic_events.jsonl".to_string());
    }
    if let Some(diag) = break_on_diagnostic {
        if supports_option(capabilities, "--break-on-diagnostic") {
            parts.push("--break-on-diagnostic".to_string());
            parts.push(shell_arg(diag));
        }
    }
    if let Some(path) = json_out {
        if supports_option(capabilities, "--json") {
            parts.push("--json".to_string());
            parts.push(shell_arg(path));
        }
    }
    if let Some(path) = snapshot_out {
        if supports_option(capabilities, "--snapshot") {
            parts.push("--snapshot".to_string());
            parts.push(shell_arg(path));
        } else if supports_option(capabilities, "--snapshot-on-diagnostic") {
            parts.push("--snapshot-on-diagnostic".to_string());
            parts.push("automation/snapshots".to_string());
        }
    }
    if let Some(path) = trace_out {
        if supports_option(capabilities, "--trace-jsonl") {
            parts.push("--trace-jsonl".to_string());
            parts.push(shell_arg(path));
        }
    }
    parts.join(" ")
}

// Match advertised command names exactly. Missing capability information is
// treated as permissive; this does not test whether the executable supports it.
fn supports_command(capabilities: Option<&ToolCapabilities>, command: &str) -> bool {
    capabilities
        .map(|cap| cap.commands.iter().any(|value| value == command))
        .unwrap_or(true)
}

// Match advertised option names exactly; absent capability information permits
// all options, without probing an executable or interpreting version numbers.
fn supports_option(capabilities: Option<&ToolCapabilities>, option: &str) -> bool {
    capabilities
        .map(|cap| cap.options.iter().any(|value| value == option))
        .unwrap_or(true)
}

// Return the same assumed CLI feature set for any tool name. This is a static
// fallback table, not the result of querying the named emulator.
fn default_capabilities(tool: &str) -> ToolCapabilities {
    ToolCapabilities {
        schema: Some("sarakura-tool-capabilities".to_string()),
        schema_version: Some(1),
        tool: tool.to_string(),
        commands: vec!["run".to_string()],
        options: vec![
            "--frames",
            "--json",
            "--emit-diagnostics",
            "--diagnostics-jsonl",
            "--png",
            "--snapshot",
            "--trace-jsonl",
            "--repro-bundle",
            "--break-on-diagnostic",
            "--png-on-diagnostic",
            "--snapshot-on-diagnostic",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        notes: vec!["Default SARAKURA v1.0 KOKURA/KUROSAKI CLI capability table.".to_string()],
    }
}

// Apply the legacy display quoting used by generated command proposals. It is
// not general shell escaping: angle brackets and empty strings remain bare,
// and double quotes do not protect shell expansion in every supported host.
fn shell_arg(value: &str) -> String {
    if value.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '\\' | ':' | '<' | '>')
    }) {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}

// Deduplicate diagnostic type strings exactly and return them in sorted order
// so retest command IDs are stable for the same set of types.
fn unique_event_types(doc: &AiDiagnosticsDocument) -> Vec<String> {
    let mut set = BTreeSet::new();
    for diag in &doc.diagnostics {
        set.insert(diag.diagnostic_type.clone());
    }
    set.into_iter().collect()
}

// Lowercase a supplied tool hint. Auto or an empty hint selects kurosaki only
// for the exact platform fc; other platforms select kokura. Unknown hints pass
// through lowercased, without path validation or whitespace trimming.
fn resolve_tool(platform: &str, tool_hint: Option<&str>) -> String {
    match tool_hint.unwrap_or("auto").to_ascii_lowercase().as_str() {
        "kokura" => "kokura".to_string(),
        "kurosaki" => "kurosaki".to_string(),
        "auto" | "" => match platform {
            "fc" => "kurosaki".to_string(),
            _ => "kokura".to_string(),
        },
        other => other.to_string(),
    }
}

// Keep ASCII letters/digits, underscore and hyphen for an output-name suffix;
// replace each other character with underscore. Different inputs can collide.
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AiDiagnostic, DiagnosticSummary, RepairTarget, RetestCondition, RomSummary, RunSummary,
        SourceMapping,
    };

    // Construct one synthetic mapped GB diagnostic for plan-structure tests;
    // these metadata fields are fixtures, not a live emulator observation.
    fn doc() -> AiDiagnosticsDocument {
        AiDiagnosticsDocument {
            schema: "sarakura-gb-ai-diagnostics".to_string(),
            schema_version: 1,
            producer: "sarakura".to_string(),
            platform: "gb".to_string(),
            build_id: Some("b1".to_string()),
            rom: RomSummary {
                path: Some("game.gb".to_string()),
                target: Some("gbc".to_string()),
                hash: None,
            },
            run: RunSummary {
                frames_requested: 300,
                diagnostic_summary_limit: 200,
                events_loaded: 1,
                events_aggregated: 1,
                diagnostics_before_filter: 1,
                catalog_rules_total: 48,
                event_type_filter: vec![],
                phase_filter: vec![],
                pack_filter: vec![],
                min_severity: None,
            },
            summary: DiagnosticSummary::default(),
            diagnostics: vec![AiDiagnostic {
                diagnostic_id: "diag_0001".to_string(),
                catalog_id: Some("GBD001".to_string()),
                diagnostic_type: "VRAM_WRITE_OUTSIDE_SAFE_PERIOD".to_string(),
                severity: "error".to_string(),
                phase: Some("MVP-1".to_string()),
                category: Some("PPU/VRAM".to_string()),
                confidence: 0.9,
                source_mapping: Some(SourceMapping {
                    source_file: Some("main.c".to_string()),
                    source_line: Some(42),
                    function_id: Some("func_draw".to_string()),
                    function_name: Some("draw".to_string()),
                    symbol_name: None,
                    op_id: Some("op1".to_string()),
                    pc: Some("0x415a".to_string()),
                    bank: Some(1),
                    prg_bank: None,
                    mapping_confidence: 0.9,
                }),
                evidence_chain: vec![],
                repair_target: RepairTarget {
                    target_type: "vblank_queue".to_string(),
                    primary_file: Some("main.c".to_string()),
                    primary_line: Some(42),
                    function_id: Some("func_draw".to_string()),
                    function_name: Some("draw".to_string()),
                    op_id: Some("op1".to_string()),
                    reason: "test".to_string(),
                },
                target_candidates: vec![],
                safe_patch_hint: "Use VBlank queue".to_string(),
                retest_condition: RetestCondition {
                    frames: 300,
                    expect_absent: vec!["VRAM_WRITE_OUTSIDE_SAFE_PERIOD".to_string()],
                    expect_not_worse: vec![],
                    required_outputs: vec![],
                },
                event_count: 1,
                first_seen: Some(10),
                last_seen: Some(10),
                snapshot_ref: None,
                trace_window_ref: None,
                sample_event_ids: vec!["evt_1".to_string()],
            }],
        }
    }

    #[test]
    // Check automatic GB tool selection and the proposed reproduction flag.
    // The test inspects generated strings and does not run the emulator.
    fn builds_kokura_plan_for_gb() {
        let plan = build_automation_plan(&doc(), None);
        assert_eq!(plan.tool, "kokura");
        assert_eq!(plan.summary.reproduce_commands, 1);
        assert!(plan.commands[0].cli.contains("kokura run"));
        assert!(plan.commands[0].cli.contains("--break-on-diagnostic"));
    }

    #[test]
    // Check that an explicit tool hint appears in English Markdown command output.
    fn markdown_contains_commands() {
        let plan = build_automation_plan(&doc(), Some("kurosaki"));
        let md = render_automation_plan_markdown(&plan);
        assert!(md.contains("SARAKURA automation plan"));
        assert!(md.contains("kurosaki run"));
    }

    #[test]
    // Check omission of unsupported diagnostic-break and snapshot flags from the
    // reproduction proposal; this does not validate every inspection branch.
    fn capabilities_filter_unsupported_options() {
        let capabilities = ToolCapabilities {
            tool: "kokura".to_string(),
            commands: vec!["run".to_string()],
            options: vec!["--frames".to_string()],
            ..ToolCapabilities::default()
        };
        let plan =
            build_automation_plan_with_capabilities(&doc(), Some("kokura"), Some(&capabilities));
        assert!(!plan.commands[0].cli.contains("--break-on-diagnostic"));
        assert!(!plan.commands[0].cli.contains("--snapshot"));
    }
}

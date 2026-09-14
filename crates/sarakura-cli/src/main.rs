use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use sarakura_core::{
    build_automation_plan, build_automation_plan_with_capabilities, build_baseline_delta,
    build_catalog_coverage, build_ci_summary, build_pack_plan, build_repair_plan,
    build_retest_plan, inspect_events, inspect_metadata, load_events_jsonl, load_metadata,
    normalize_events, render_automation_plan_markdown, render_baseline_delta_markdown,
    render_repair_plan_markdown, validate_emitter_compatibility_with_options, write_ai_diagnostics,
    write_diagnostic_summary, write_events_jsonl, write_repair_prompt, write_retest_plan,
    AiDiagnosticsDocument, AnalyzeOptions, BuildMetadata, EmitterCompatibilityOptions, Platform,
    ReproBundleEntry, ReproBundleInspection, ToolCapabilities,
};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "sarakura",
    version,
    about = "SARAKURA diagnostic translation CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Gb(PlatformCommand),
    Fc(PlatformCommand),
    Validate(ValidateArgs),
    Compare(CompareArgs),
    Catalog(CatalogArgs),
    InspectEvents(InspectEventsArgs),
    InspectMetadata(InspectMetadataArgs),
    RetestPlan(RetestPlanArgs),
    EmitterCheck(EmitterCheckArgs),
    NormalizeEvents(NormalizeEventsArgs),
    CiSummary(CiSummaryArgs),
    Coverage(CoverageArgs),
    PackPlan(PackPlanArgs),
    RepairPlan(RepairPlanArgs),
    AutomationPlan(AutomationPlanArgs),
    BaselineDelta(BaselineDeltaArgs),
    Schema(SchemaArgs),
    InspectRepro(InspectReproArgs),
}

#[derive(Debug, Args)]
struct PlatformCommand {
    #[command(subcommand)]
    command: AnalyzeCommand,
}

#[derive(Debug, Subcommand)]
enum AnalyzeCommand {
    Analyze(AnalyzeArgs),
}

#[derive(Debug, Args)]
// Clap defines file inputs, a recorded frame budget, filters and output policy.
// Repeated selectors are passed to the core as lists; frames does not run a ROM.
struct AnalyzeArgs {
    #[arg(long)]
    metadata: PathBuf,
    #[arg(long)]
    events: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value_t = 300)]
    frames: u64,
    #[arg(long, default_value_t = 200)]
    diagnostic_summary_limit: usize,
    /// Analyze only this event_type or catalog ID. Can be repeated.
    #[arg(long = "diagnostic-rule")]
    diagnostic_rules: Vec<String>,
    /// Analyze only this phase, for example MVP-1. Can be repeated.
    #[arg(long)]
    phase: Vec<String>,
    /// Analyze only this diagnostic pack/domain, for example core, ppu, audio, mapper, fds. Can be repeated.
    #[arg(long = "diagnostic-pack")]
    diagnostic_packs: Vec<String>,
    /// Keep only diagnostics at or above this severity.
    #[arg(long)]
    min_severity: Option<SeverityArg>,
    /// Return a non-zero exit code if diagnostics at or above this severity remain.
    #[arg(long, default_value_t = FailOn::Never)]
    fail_on: FailOn,
    /// Do not emit report.html.
    #[arg(long)]
    no_report: bool,
    /// Do not emit repro_bundle.zip.
    #[arg(long)]
    no_repro_bundle: bool,
    /// Preserve project build labels, paths, and source identifiers in output.
    #[arg(long)]
    allow_project_labels: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SeverityArg {
    Error,
    Warn,
    Info,
}

impl SeverityArg {
    // Map the parsed severity enum to the lowercase spelling expected by core
    // filters and serialized diagnostic policies.
    fn as_str(self) -> &'static str {
        match self {
            SeverityArg::Error => "error",
            SeverityArg::Warn => "warn",
            SeverityArg::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FailOn {
    Never,
    Error,
    Warn,
    Info,
}

impl std::fmt::Display for FailOn {
    // Use stable lowercase policy names for Clap defaults, help and CI-plan input.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailOn::Never => write!(f, "never"),
            FailOn::Error => write!(f, "error"),
            FailOn::Warn => write!(f, "warn"),
            FailOn::Info => write!(f, "info"),
        }
    }
}

#[derive(Debug, Args)]
struct ValidateArgs {
    path: PathBuf,
    #[arg(long)]
    strict: bool,
}

#[derive(Debug, Args)]
struct CompareArgs {
    /// Directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    before: PathBuf,
    /// Directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    after: PathBuf,
    #[arg(long)]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct CatalogArgs {
    #[arg(value_enum)]
    platform: CatalogPlatform,
    /// Emit JSON instead of a Markdown table.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct InspectEventsArgs {
    #[arg(long)]
    events: PathBuf,
    /// Optional path to write the JSON inspection summary.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Emit JSON to stdout instead of a compact text summary.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct InspectMetadataArgs {
    #[arg(long)]
    metadata: PathBuf,
    /// Optional path to write the JSON inspection summary.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Emit JSON to stdout instead of a compact text summary.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RetestPlanArgs {
    /// Directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    diagnostics: PathBuf,
    /// Optional path to write retest_plan.json. If omitted, JSON is printed to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct EmitterCheckArgs {
    #[arg(value_enum)]
    platform: CatalogPlatform,
    #[arg(long)]
    metadata: Option<PathBuf>,
    #[arg(long)]
    events: PathBuf,
    /// Fail when compatibility warnings are found.
    #[arg(long)]
    strict: bool,
    /// Check that snapshot_ref paths exist.
    #[arg(long)]
    check_snapshot_refs: bool,
    /// Check that trace_window_ref paths exist.
    #[arg(long)]
    check_trace_window_refs: bool,
    /// Base directory for snapshot_ref and trace_window_ref existence checks.
    #[arg(long)]
    base_dir: Option<PathBuf>,
    /// Optional path to write the compatibility summary JSON.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct NormalizeEventsArgs {
    #[arg(long)]
    events: PathBuf,
    #[arg(long)]
    out: PathBuf,
    /// Also write a pretty JSON array next to the JSONL output for fixture review.
    #[arg(long)]
    json_array: bool,
}

#[derive(Debug, Args)]
struct CiSummaryArgs {
    /// Directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    diagnostics: PathBuf,
    /// CI policy. `error` fails only on errors, `warn` fails on errors or warnings.
    #[arg(long, default_value_t = FailOn::Error)]
    fail_on: FailOn,
    /// Optional path to write sarakura_ci_summary.json.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Return a non-zero exit code when the CI policy fails.
    #[arg(long)]
    enforce: bool,
}

#[derive(Debug, Args)]
struct CoverageArgs {
    #[arg(value_enum)]
    platform: CatalogPlatform,
    #[arg(long)]
    events: PathBuf,
    /// Optional path to write catalog coverage JSON. If a directory is given, diagnostic_coverage.json is created inside it.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Emit JSON to stdout instead of a compact text summary.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct PackPlanArgs {
    #[arg(value_enum)]
    platform: CatalogPlatform,
    /// Optional path to write pack plan JSON. If a directory is given, diagnostic_pack_plan.json is created inside it.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Emit JSON to stdout instead of a Markdown summary.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RepairPlanArgs {
    /// Directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    diagnostics: PathBuf,
    /// Optional path to write repair_plan.json. If omitted, JSON is printed to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Optional path to write repair_plan.md. If a directory is given, repair_plan.md is created inside it.
    #[arg(long)]
    markdown: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct AutomationPlanArgs {
    /// Directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    diagnostics: PathBuf,
    /// Tool name for generated commands. Use auto, kokura, or kurosaki.
    #[arg(long, default_value = "auto")]
    tool: String,
    /// Optional path to write automation_plan.json. If omitted, JSON is printed to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Optional path to write automation_plan.md. If a directory is given, automation_plan.md is created inside it.
    #[arg(long)]
    markdown: Option<PathBuf>,
    /// Optional SARAKURA tool capability JSON. Unsupported options are omitted from generated commands.
    #[arg(long)]
    capabilities: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct BaselineDeltaArgs {
    /// Baseline directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    baseline: PathBuf,
    /// Current directory containing ai_diagnostics.json, or an ai_diagnostics.json file.
    #[arg(long)]
    current: PathBuf,
    /// Optional path to write baseline_delta.json. If omitted, JSON is printed to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Optional path to write baseline_delta.md. If a directory is given, baseline_delta.md is created inside it.
    #[arg(long)]
    markdown: Option<PathBuf>,
    /// Fail when new diagnostics at or above this severity are introduced.
    #[arg(long, default_value_t = FailOn::Error)]
    fail_on_new: FailOn,
    /// Fail when existing diagnostics regress at or above this severity.
    #[arg(long, default_value_t = FailOn::Error)]
    fail_on_regression: FailOn,
    /// Return a non-zero exit code when the baseline policy fails.
    #[arg(long)]
    enforce: bool,
}

#[derive(Debug, Args)]
struct SchemaArgs {
    #[command(subcommand)]
    command: SchemaCommand,
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    Export(SchemaExportArgs),
}

#[derive(Debug, Args)]
struct SchemaExportArgs {
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    manifest: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct InspectReproArgs {
    bundle: PathBuf,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CatalogPlatform {
    Gb,
    Fc,
}

// Let Clap parse the command tree, then dispatch one operation. Returned
// errors become a nonzero process exit; analysis and plan commands consume
// existing files and do not launch an emulator or apply source patches.
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Gb(cmd) => match cmd.command {
            AnalyzeCommand::Analyze(args) => run_analyze_gb(args),
        },
        Command::Fc(cmd) => match cmd.command {
            AnalyzeCommand::Analyze(args) => run_analyze_fc(args),
        },
        Command::Validate(args) => {
            sarakura_schema::validate_ai_diagnostics_file_with_options(&args.path, args.strict)?;
            println!("OK: {}", args.path.display());
            Ok(())
        }
        Command::Compare(args) => run_compare(args),
        Command::Catalog(args) => run_catalog(args),
        Command::InspectEvents(args) => run_inspect_events(args),
        Command::InspectMetadata(args) => run_inspect_metadata(args),
        Command::RetestPlan(args) => run_retest_plan(args),
        Command::EmitterCheck(args) => run_emitter_check(args),
        Command::NormalizeEvents(args) => run_normalize_events(args),
        Command::CiSummary(args) => run_ci_summary(args),
        Command::Coverage(args) => run_coverage(args),
        Command::PackPlan(args) => run_pack_plan(args),
        Command::RepairPlan(args) => run_repair_plan(args),
        Command::AutomationPlan(args) => run_automation_plan(args),
        Command::BaselineDelta(args) => run_baseline_delta(args),
        Command::Schema(args) => run_schema(args),
        Command::InspectRepro(args) => run_inspect_repro(args),
    }
}

// Copy the CLI filters and disclosure setting into core options. Output
// selection and fail-on policy stay in the CLI and are handled after analysis.
fn analyze_options(args: &AnalyzeArgs, platform: Platform) -> AnalyzeOptions {
    AnalyzeOptions {
        platform,
        frames: args.frames,
        summary_limit: args.diagnostic_summary_limit,
        event_type_filter: args.diagnostic_rules.clone(),
        phase_filter: args.phase.clone(),
        pack_filter: args.diagnostic_packs.clone(),
        min_severity: args.min_severity.map(|s| s.as_str().to_string()),
        allow_project_labels: args.allow_project_labels,
    }
}

// Load metadata and JSONL events, analyze with the GB catalog, then write
// the requested artifacts before applying the CLI failure policy.
fn run_analyze_gb(args: AnalyzeArgs) -> Result<()> {
    let metadata = load_metadata(&args.metadata)?;
    let events = load_events_jsonl(&args.events)?;
    let options = analyze_options(&args, Platform::Gb);
    let doc = sarakura_gb::analyze_gb_with_options(metadata, events, options)?;
    write_outputs(args, &doc)
}

// Load the FC input files and run catalog analysis with the selected filters.
// Artifact generation and the exit policy are shared with the GB path.
fn run_analyze_fc(args: AnalyzeArgs) -> Result<()> {
    let metadata = load_metadata(&args.metadata)?;
    let events = load_events_jsonl(&args.events)?;
    let options = analyze_options(&args, Platform::Fc);
    let doc = sarakura_fc::analyze_fc_with_options(metadata, events, options)?;
    write_outputs(args, &doc)
}

// Write derived JSON, Markdown and optional HTML/ZIP artifacts in sequence.
// A failure can leave earlier outputs in place. Apply fail-on only after writing
// so a policy failure still leaves diagnostics available for inspection.
fn write_outputs(args: AnalyzeArgs, doc: &AiDiagnosticsDocument) -> Result<()> {
    fs::create_dir_all(&args.out)
        .with_context(|| format!("failed to create {}", args.out.display()))?;
    write_ai_diagnostics(args.out.join("ai_diagnostics.json"), doc)?;
    write_diagnostic_summary(args.out.join("diagnostic_summary.json"), doc)?;
    write_repair_prompt(args.out.join("repair_prompt.md"), doc)?;
    write_retest_plan(args.out.join("retest_plan.json"), doc)?;
    let repair_plan = build_repair_plan(doc);
    write_json_file(&args.out.join("repair_plan.json"), &repair_plan)?;
    fs::write(
        args.out.join("repair_plan.md"),
        render_repair_plan_markdown(&repair_plan),
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            args.out.join("repair_plan.md").display()
        )
    })?;
    let automation_plan = build_automation_plan(doc, None);
    write_json_file(&args.out.join("automation_plan.json"), &automation_plan)?;
    fs::write(
        args.out.join("automation_plan.md"),
        render_automation_plan_markdown(&automation_plan),
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            args.out.join("automation_plan.md").display()
        )
    })?;
    // This flag suppresses the standalone HTML file. An enabled repro ZIP still
    // contains its own report.html generated by the bundle writer.
    if !args.no_report {
        sarakura_report::write_html_report(args.out.join("report.html"), doc)?;
    }
    if !args.no_repro_bundle {
        sarakura_report::write_repro_bundle(
            args.out.join("repro_bundle.zip"),
            &[args.metadata.clone(), args.events.clone()],
            doc,
        )?;
    }
    println!(
        "wrote SARAKURA diagnostics to {} (total={}, errors={}, warnings={}, infos={})",
        args.out.display(),
        doc.summary.diagnostics_total,
        doc.summary.errors,
        doc.summary.warnings,
        doc.summary.infos
    );
    enforce_fail_on(args.fail_on, doc)
}

// Gate the exit status using the stored, filtered and limited summary. Warn
// includes errors, info includes every retained diagnostic, and never disables
// the gate. Dropped diagnostics are not recovered or counted here.
fn enforce_fail_on(fail_on: FailOn, doc: &AiDiagnosticsDocument) -> Result<()> {
    match fail_on {
        FailOn::Never => Ok(()),
        FailOn::Error if doc.summary.errors > 0 => bail!(
            "SARAKURA fail-on=error: {} error diagnostics remain",
            doc.summary.errors
        ),
        FailOn::Warn if doc.summary.errors + doc.summary.warnings > 0 => bail!(
            "SARAKURA fail-on=warn: {} error/warn diagnostics remain",
            doc.summary.errors + doc.summary.warnings
        ),
        FailOn::Info if doc.summary.diagnostics_total > 0 => bail!(
            "SARAKURA fail-on=info: {} diagnostics remain",
            doc.summary.diagnostics_total
        ),
        _ => Ok(()),
    }
}

// Resolve two diagnostics inputs and write a count/type comparison into the
// output directory. Use baseline-delta for source-group regression policies.
fn run_compare(args: CompareArgs) -> Result<()> {
    fs::create_dir_all(&args.out)
        .with_context(|| format!("failed to create {}", args.out.display()))?;
    let before = read_ai_diagnostics(&args.before)?;
    let after = read_ai_diagnostics(&args.after)?;
    let md = sarakura_report::compare_documents(&before, &after);
    fs::write(args.out.join("comparison_report.md"), md).with_context(|| {
        format!(
            "failed to write {}",
            args.out.join("comparison_report.md").display()
        )
    })?;
    println!("wrote comparison report to {}", args.out.display());
    Ok(())
}

// Print the selected built-in rule catalog as JSON or a Markdown table.
// The table uses embedded catalog labels directly without Markdown escaping.
fn run_catalog(args: CatalogArgs) -> Result<()> {
    let rules = match args.platform {
        CatalogPlatform::Gb => sarakura_gb::gb_catalog()?,
        CatalogPlatform::Fc => sarakura_fc::fc_catalog()?,
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&rules)?);
    } else {
        println!("| ID | event_type | Phase | Severity | Category |");
        println!("|---|---|---|---|---|");
        for rule in rules {
            println!(
                "| {} | `{}` | {} | {} | {} |",
                rule.id, rule.event_type, rule.phase, rule.severity, rule.category
            );
        }
    }
    Ok(())
}

// Summarize the loaded events, optionally write JSON at the exact output path,
// and always print either the JSON summary or compact counts to stdout.
fn run_inspect_events(args: InspectEventsArgs) -> Result<()> {
    let events = load_events_jsonl(&args.events)?;
    let summary = inspect_events(&events);
    if let Some(out) = args.out {
        write_json_file(&out, &summary)?;
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "events={} effective_count={} types={} frame_range={:?}-{:?}",
            summary.events_loaded,
            summary.effective_count,
            summary.event_types.len(),
            summary.frames.first,
            summary.frames.last
        );
    }
    Ok(())
}

// Inspect metadata fields without running analysis. An optional JSON file
// does not suppress the independently selected stdout summary.
fn run_inspect_metadata(args: InspectMetadataArgs) -> Result<()> {
    let metadata = load_metadata(&args.metadata)?;
    let summary = inspect_metadata(&metadata);
    if let Some(out) = args.out {
        write_json_file(&out, &summary)?;
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "schema={:?} build_id={:?} target={:?} arrays={}",
            summary.schema,
            summary.build_id,
            summary.target,
            summary.array_counts.len()
        );
    }
    Ok(())
}

// Build a retest proposal from a diagnostics file or directory. With --out,
// write to that exact path; otherwise print JSON. No proposed command is run.
fn run_retest_plan(args: RetestPlanArgs) -> Result<()> {
    let doc = read_ai_diagnostics(&args.diagnostics)?;
    if let Some(out) = args.out {
        write_retest_plan(&out, &doc)?;
        println!("wrote retest plan to {}", out.display());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_retest_plan(&doc))?
        );
    }
    Ok(())
}

// Check loaded event/metadata compatibility for the selected emitter family.
// Reference existence checks are optional. Write any requested report before
// strict mode converts compatibility warnings into a failing exit status.
fn run_emitter_check(args: EmitterCheckArgs) -> Result<()> {
    let metadata = args.metadata.as_ref().map(load_metadata).transpose()?;
    let events = load_events_jsonl(&args.events)?;
    let expected_schema_prefix = match args.platform {
        CatalogPlatform::Gb => "kitaqgb",
        CatalogPlatform::Fc => "kitaqfc",
    };
    let warnings = validate_emitter_compatibility_with_options(
        metadata.as_ref(),
        &events,
        &EmitterCompatibilityOptions {
            expected_schema_prefix: expected_schema_prefix.to_string(),
            check_snapshot_refs: args.check_snapshot_refs,
            check_trace_window_refs: args.check_trace_window_refs,
            base_dir: args.base_dir.clone(),
        },
    );
    let metadata_summary = metadata
        .as_ref()
        .map(inspect_metadata)
        .unwrap_or_else(|| inspect_metadata(&BuildMetadata::default()));
    let event_summary = inspect_events(&events);
    let report = serde_json::json!({
        "schema": "sarakura-emitter-compatibility-report",
        "schema_version": 1,
        "platform": match args.platform { CatalogPlatform::Gb => "gb", CatalogPlatform::Fc => "fc" },
        "warnings": warnings,
        "metadata": metadata_summary,
        "events": event_summary
    });
    if let Some(out) = args.out {
        write_json_file(&out, &report)?;
    }
    let warning_count = report
        .get("warnings")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    println!("emitter compatibility warnings={}", warning_count);
    if args.strict && warning_count > 0 {
        bail!(
            "emitter compatibility check failed with {} warning(s)",
            warning_count
        );
    }
    Ok(())
}

// Normalize and aggregate the loaded events, then write JSONL. The optional
// array uses the same output stem with a .json extension; a later write failure
// does not roll back the JSONL file.
fn run_normalize_events(args: NormalizeEventsArgs) -> Result<()> {
    let events = load_events_jsonl(&args.events)?;
    let normalized = normalize_events(&events);
    write_events_jsonl(&args.out, &normalized)?;
    if args.json_array {
        let array_path = args.out.with_extension("json");
        write_json_file(&array_path, &normalized)?;
    }
    println!(
        "normalized events: input={} output={} written={}",
        events.len(),
        normalized.len(),
        args.out.display()
    );
    Ok(())
}

// Compute a policy result from stored diagnostics and emit JSON. Only --enforce
// turns a failed policy into a command error; report-only mode still exits normally.
fn run_ci_summary(args: CiSummaryArgs) -> Result<()> {
    let doc = read_ai_diagnostics(&args.diagnostics)?;
    let summary = build_ci_summary(&doc, &args.fail_on.to_string());
    if let Some(out) = args.out {
        write_json_file(&out, &summary)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    }
    if args.enforce && summary.exit_code != 0 {
        bail!("{}", summary.message);
    }
    Ok(())
}

// Compare observed event names with the selected catalog and emit coverage.
// This measures supplied observations, not whether every emitter or rule works.
fn run_coverage(args: CoverageArgs) -> Result<()> {
    let events = load_events_jsonl(&args.events)?;
    let catalog = match args.platform {
        CatalogPlatform::Gb => sarakura_gb::gb_catalog()?,
        CatalogPlatform::Fc => sarakura_fc::fc_catalog()?,
    };
    let platform = match args.platform {
        CatalogPlatform::Gb => Platform::Gb,
        CatalogPlatform::Fc => Platform::Fc,
    };
    let report = build_catalog_coverage(platform, &catalog, &events);
    if let Some(out) = args.out {
        write_json_file(
            &resolve_output_json_path(&out, "diagnostic_coverage.json"),
            &report,
        )?;
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "coverage platform={} observed_rules={}/{} missing={} unknown_event_types={}",
            report.platform,
            report.observed_catalog_rules,
            report.catalog_rules_total,
            report.missing_catalog_rules,
            report.unknown_event_types.len()
        );
    }
    Ok(())
}

// Group the selected catalog into recommended packs, phases and domains.
// Optionally save JSON and print either JSON or the English Markdown summary.
fn run_pack_plan(args: PackPlanArgs) -> Result<()> {
    let catalog = match args.platform {
        CatalogPlatform::Gb => sarakura_gb::gb_catalog()?,
        CatalogPlatform::Fc => sarakura_fc::fc_catalog()?,
    };
    let platform = match args.platform {
        CatalogPlatform::Gb => Platform::Gb,
        CatalogPlatform::Fc => Platform::Fc,
    };
    let plan = build_pack_plan(platform, &catalog);
    if let Some(out) = args.out {
        write_json_file(
            &resolve_output_json_path(&out, "diagnostic_pack_plan.json"),
            &plan,
        )?;
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("# SARAKURA diagnostic packs ({})", plan.platform);
        println!();
        println!("## Recommended packs");
        for pack in &plan.recommended_packs {
            println!(
                "- `{}`: {} rules — {}",
                pack.name,
                pack.rule_ids.len(),
                pack.description
            );
        }
        println!();
        println!("## Phases");
        for (phase, rule_ids) in &plan.phases {
            println!("- `{}`: {} rules", phase, rule_ids.len());
        }
        println!();
        println!("## Domains");
        for (domain, rule_ids) in &plan.domains {
            println!("- `{}`: {} rules", domain, rule_ids.len());
        }
    }
    Ok(())
}

// Create a repair proposal and optional JSON/Markdown files. Print JSON when
// no JSON output is requested, even if Markdown is written; never apply patches.
fn run_repair_plan(args: RepairPlanArgs) -> Result<()> {
    let doc = read_ai_diagnostics(&args.diagnostics)?;
    let plan = build_repair_plan(&doc);
    if let Some(out) = &args.out {
        write_json_file(&resolve_output_json_path(out, "repair_plan.json"), &plan)?;
    }
    if let Some(markdown) = &args.markdown {
        let markdown_path = resolve_output_md_path(markdown, "repair_plan.md");
        if let Some(parent) = markdown_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&markdown_path, render_repair_plan_markdown(&plan))
            .with_context(|| format!("failed to write {}", markdown_path.display()))?;
    }
    if args.out.is_none() {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!(
            "repair plan steps={} diagnostics_covered={}",
            plan.summary.total_steps, plan.summary.diagnostics_covered
        );
    }
    Ok(())
}

// Load optional advertised capabilities and generate command proposals.
// Whitespace-only tool hints select the core default. Write requested formats
// without executing or validating the proposals against installed tools.
fn run_automation_plan(args: AutomationPlanArgs) -> Result<()> {
    let doc = read_ai_diagnostics(&args.diagnostics)?;
    let tool_hint = if args.tool.trim().is_empty() {
        None
    } else {
        Some(args.tool.as_str())
    };
    let capabilities = args
        .capabilities
        .as_ref()
        .map(|path| read_tool_capabilities(path))
        .transpose()?;
    let plan = build_automation_plan_with_capabilities(&doc, tool_hint, capabilities.as_ref());
    if let Some(out) = &args.out {
        write_json_file(
            &resolve_output_json_path(out, "automation_plan.json"),
            &plan,
        )?;
    }
    if let Some(markdown) = &args.markdown {
        let markdown_path = resolve_output_md_path(markdown, "automation_plan.md");
        if let Some(parent) = markdown_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&markdown_path, render_automation_plan_markdown(&plan))
            .with_context(|| format!("failed to write {}", markdown_path.display()))?;
    }
    if args.out.is_none() {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!(
            "automation plan commands={} tool={}",
            plan.summary.total_commands, plan.tool
        );
    }
    Ok(())
}

// Export the named schema files and optionally write a manifest of returned
// paths. A manifest failure leaves the schema files already written.
fn run_schema(args: SchemaArgs) -> Result<()> {
    match args.command {
        SchemaCommand::Export(args) => {
            let written = sarakura_schema::write_schema_bundle(&args.out)?;
            if let Some(manifest) = args.manifest {
                let manifest_value = serde_json::json!({
                    "schema": "sarakura-schema-export-manifest",
                    "schema_version": 1,
                    "schemas": written,
                });
                write_json_file(&manifest, &manifest_value)?;
            }
            println!(
                "wrote {} SARAKURA schema files to {}",
                written.len(),
                args.out.display()
            );
            Ok(())
        }
    }
}

// Inspect ZIP metadata and the manifest without extracting entries. JSON is
// printed by default when no output file is requested; otherwise --json controls
// stdout detail independently of the JSON file.
fn run_inspect_repro(args: InspectReproArgs) -> Result<()> {
    let inspection = inspect_repro_bundle(&args.bundle)?;
    if let Some(out) = &args.out {
        write_json_file(out, &inspection)?;
    }
    if args.json || args.out.is_none() {
        println!("{}", serde_json::to_string_pretty(&inspection)?);
    } else {
        println!(
            "repro bundle entries={} manifest_present={} warnings={}",
            inspection.entries.len(),
            inspection.manifest_present,
            inspection.warnings.len()
        );
    }
    Ok(())
}

// Compare grouped diagnostics under separate new/regression policies, emit
// requested JSON/Markdown, then fail only when enforcement is enabled and the
// computed policy fails. This does not rerun either measured workload.
fn run_baseline_delta(args: BaselineDeltaArgs) -> Result<()> {
    let baseline = read_ai_diagnostics(&args.baseline)?;
    let current = read_ai_diagnostics(&args.current)?;
    let report = build_baseline_delta(
        &baseline,
        &current,
        &args.fail_on_new.to_string(),
        &args.fail_on_regression.to_string(),
    );
    if let Some(out) = &args.out {
        write_json_file(
            &resolve_output_json_path(out, "baseline_delta.json"),
            &report,
        )?;
    }
    if let Some(markdown) = &args.markdown {
        let markdown_path = resolve_output_md_path(markdown, "baseline_delta.md");
        if let Some(parent) = markdown_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&markdown_path, render_baseline_delta_markdown(&report))
            .with_context(|| format!("failed to write {}", markdown_path.display()))?;
    }
    if args.out.is_none() {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "baseline delta status={} new={} resolved={} regressed={} improved={}",
            report.status,
            report.summary.new_total,
            report.summary.resolved_total,
            report.summary.regressed_total,
            report.summary.improved_total
        );
    }
    if args.enforce && report.exit_code != 0 {
        bail!("{}", report.message);
    }
    Ok(())
}

// Accept an existing directory or explicit file, read UTF-8 JSON, and
// deserialize the document with path context. This does not call strict validation.
fn read_ai_diagnostics(path: &Path) -> Result<AiDiagnosticsDocument> {
    let path = resolve_ai_diagnostics_path(path);
    serde_json::from_str(
        &fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))
}

// Read an explicit JSON capability file into the core model. The advertised
// commands and options are not probed against the named executable.
fn read_tool_capabilities(path: &Path) -> Result<ToolCapabilities> {
    serde_json::from_str(
        &fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))
}

// List ZIP entry names and declared sizes, parsing entries named manifest.json.
// Warn about missing expected names; do not extract or validate report contents.
// If several manifests exist, the last parsed value is retained.
fn inspect_repro_bundle(path: &Path) -> Result<ReproBundleInspection> {
    let file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read zip {}", path.display()))?;
    let mut entries = Vec::new();
    let mut manifest = None;
    let mut warnings = Vec::new();
    for index in 0..zip.len() {
        let mut file = zip.by_index(index)?;
        let name = file.name().to_string();
        let size = file.size();
        // Read only the manifest body. Other entries are listed by metadata, without
        // extracting their paths or reading/decompressing their contents.
        if name == "manifest.json" {
            let mut text = String::new();
            file.read_to_string(&mut text)?;
            manifest = Some(serde_json::from_str(&text).context("failed to parse manifest.json")?);
        }
        entries.push(ReproBundleEntry { name, size });
    }
    let names = entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for required in [
        "manifest.json",
        "ai_diagnostics.json",
        "retest_plan.json",
        "repair_plan.json",
        "automation_plan.json",
    ] {
        if !names.contains(required) {
            warnings.push(format!("missing expected repro bundle entry: {required}"));
        }
    }
    Ok(ReproBundleInspection {
        schema: "sarakura-repro-bundle-inspection".to_string(),
        schema_version: 1,
        path: path.display().to_string(),
        manifest_present: manifest.is_some(),
        manifest,
        entries,
        warnings,
    })
}

// Create parent directories, serialize pretty JSON and replace the exact
// file path. Serialization/I/O errors propagate; writes are not atomic.
fn write_json_file(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value).context("failed to serialize json")?;
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

// Treat any path with an extension as a file, otherwise append the default
// JSON filename. This is a naming heuristic, not a filesystem directory check.
fn resolve_output_json_path(path: &Path, default_name: &str) -> PathBuf {
    if path.extension().is_some() {
        path.to_path_buf()
    } else {
        path.join(default_name)
    }
}

// Use an extension-bearing path verbatim or append the default Markdown
// filename. Extensionless filenames are interpreted as directories.
fn resolve_output_md_path(path: &Path, default_name: &str) -> PathBuf {
    if path.extension().is_some() {
        path.to_path_buf()
    } else {
        path.join(default_name)
    }
}

// Append ai_diagnostics.json only for an existing directory. A nonexistent
// path is used verbatim and will fail during the subsequent file read.
fn resolve_ai_diagnostics_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join("ai_diagnostics.json")
    } else {
        path.to_path_buf()
    }
}

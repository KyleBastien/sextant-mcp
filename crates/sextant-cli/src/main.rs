use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use ignore::WalkBuilder;
use sextant_config::Config;
use sextant_core::{EvalContext, Report, SourceFile, Verdict, VerdictThresholds};
use sextant_rules::RuleSet;

#[derive(Debug, Parser)]
#[command(
    name = "sextant",
    version,
    about = "Code grader for AI agent workflows"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Grade files (whole-file mode).
    Grade {
        /// Paths to grade. Defaults to the current directory.
        paths: Vec<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Severity at which to exit non-zero.
        #[arg(long, value_enum, default_value_t = FailOn::Error)]
        fail_on: FailOn,
    },
    /// Rule introspection commands.
    Rules {
        #[command(subcommand)]
        cmd: RulesCmd,
    },
}

#[derive(Debug, Subcommand)]
enum RulesCmd {
    /// List all loaded rules.
    List,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FailOn {
    Never,
    Warn,
    Error,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.cmd {
        Cmd::Grade {
            paths,
            format,
            fail_on,
        } => cmd_grade(paths, format, fail_on),
        Cmd::Rules {
            cmd: RulesCmd::List,
        } => cmd_rules_list(),
    }
}

fn cmd_grade(paths: Vec<PathBuf>, format: Format, fail_on: FailOn) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let config = Config::from_repo_root(&cwd).context("loading config")?;
    let ruleset = RuleSet::builtin(&config);

    let targets = if paths.is_empty() {
        vec![cwd.clone()]
    } else {
        paths
    };

    let files = collect_source_files(&cwd, &targets)?;
    let ctx = EvalContext { repo_root: &cwd };
    let findings = ruleset.grade_files(&files, &ctx);

    let thresholds: VerdictThresholds = (&config.verdict).into();
    let verdict = thresholds.evaluate(&findings);
    let report = Report::build(findings, verdict);

    match format {
        Format::Human => print_human(&report),
        Format::Json => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
        }
    }

    Ok(exit_for(&report, fail_on))
}

fn cmd_rules_list() -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let config = Config::from_repo_root(&cwd)?;
    let ruleset = RuleSet::builtin(&config);
    for ev in ruleset.evaluators() {
        let r = ev.rule();
        println!(
            "{}\t{}\t{}\t{}",
            r.id,
            r.severity.as_str(),
            format!("{:?}", r.scope).to_lowercase(),
            r.name
        );
    }
    Ok(ExitCode::from(0))
}

fn collect_source_files(root: &Path, targets: &[PathBuf]) -> Result<Vec<SourceFile>> {
    let mut out = Vec::new();
    for target in targets {
        let walker = WalkBuilder::new(target).standard_filters(true).build();
        for dent in walker {
            let dent = match dent {
                Ok(d) => d,
                Err(err) => {
                    tracing::warn!(?err, "walk error");
                    continue;
                }
            };
            if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let path = dent.into_path();
            let contents = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(err) => {
                    tracing::debug!(?err, ?path, "skipping unreadable");
                    continue;
                }
            };
            let abs = if path.is_absolute() {
                path
            } else {
                root.join(path)
            };
            out.push(SourceFile::new(abs, contents));
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn print_human(report: &Report) {
    if report.findings.is_empty() {
        println!("No findings.");
    } else {
        for f in &report.findings {
            let line = f.line.map(|l| format!(":{l}")).unwrap_or_default();
            println!(
                "{:<5} {}{}\t{}\t{}",
                f.severity.as_str(),
                f.path.display(),
                line,
                f.rule_id,
                f.message
            );
        }
    }
    println!();
    println!("{}", report.summary);
}

fn exit_for(report: &Report, fail_on: FailOn) -> ExitCode {
    let bad = match fail_on {
        FailOn::Never => return ExitCode::from(0),
        FailOn::Warn => report.counts.error > 0 || report.counts.warn > 0,
        FailOn::Error => report.counts.error > 0,
    };
    let verdict_blocks = matches!(report.verdict, Verdict::RequestChanges { .. });
    if bad || verdict_blocks {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

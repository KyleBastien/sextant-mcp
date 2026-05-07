use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::ValueEnum;
use sextant_core::{Report, Verdict};
use sextant_engine::{grade, DiffOptions, GradeMode};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FailOn {
    Never,
    Warn,
    Error,
}

pub struct GradeArgs {
    pub paths: Vec<PathBuf>,
    pub diff: bool,
    pub base: Option<String>,
    pub head: Option<String>,
    pub working_tree: bool,
    pub format: Format,
    pub fail_on: FailOn,
}

pub(crate) fn run(args: GradeArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let mode = if args.diff {
        GradeMode::Diff(DiffOptions {
            base: args.base,
            head: args.head,
            working_tree: args.working_tree,
        })
    } else {
        GradeMode::Files { paths: args.paths }
    };
    let report = grade(&cwd, mode).context("grading")?;

    match args.format {
        Format::Human => print_human(&report),
        Format::Json => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
        }
    }

    Ok(exit_for(&report, args.fail_on))
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

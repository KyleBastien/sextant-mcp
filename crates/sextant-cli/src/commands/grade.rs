use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::ValueEnum;
use sextant_core::{Report, Verdict};
use sextant_engine::{
    grade_pr, grade_with, DiffOptions, GradeMode, GradeOptions, PrOptions, PrReport,
};

use crate::commands::format;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Human,
    Json,
    Markdown,
    Sarif,
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
    pub pr: bool,
    pub baseline_cache: Option<PathBuf>,
    pub base: Option<String>,
    pub head: Option<String>,
    pub working_tree: bool,
    pub format: Format,
    pub output: Option<PathBuf>,
    pub fail_on: FailOn,
    pub no_llm: bool,
}

pub(crate) fn run(args: GradeArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    if args.pr {
        run_pr(&cwd, args)
    } else {
        run_normal(&cwd, args)
    }
}

fn run_normal(cwd: &std::path::Path, args: GradeArgs) -> Result<ExitCode> {
    let mode = if args.diff {
        GradeMode::Diff(DiffOptions {
            base: args.base,
            head: args.head,
            working_tree: args.working_tree,
        })
    } else {
        GradeMode::Files { paths: args.paths }
    };
    let report = grade_with(
        cwd,
        mode,
        GradeOptions {
            no_llm: args.no_llm,
        },
    )
    .context("grading")?;
    let rendered = render_normal(&report, args.format)?;
    emit(rendered, args.output.as_deref())?;
    Ok(exit_for(&report, args.fail_on))
}

fn run_pr(cwd: &std::path::Path, args: GradeArgs) -> Result<ExitCode> {
    let pr = grade_pr(
        cwd,
        DiffOptions {
            base: args.base,
            head: args.head,
            working_tree: args.working_tree,
        },
        PrOptions {
            baseline_cache: args.baseline_cache,
            grade: GradeOptions {
                no_llm: args.no_llm,
            },
        },
    )
    .context("grading PR")?;
    let rendered = render_pr(&pr, args.format)?;
    emit(rendered, args.output.as_deref())?;
    Ok(exit_for_pr(&pr, args.fail_on))
}

fn render_normal(report: &Report, format: Format) -> Result<String> {
    Ok(match format {
        Format::Human => format::human(report),
        Format::Json => format::json(report)?,
        Format::Sarif => format::sarif(report)?,
        Format::Markdown => {
            // No baseline available without --pr; fall back to the JSON
            // form so callers don't get a silent "format not applicable".
            format::json(report)?
        }
    })
}

fn render_pr(pr: &PrReport, format: Format) -> Result<String> {
    Ok(match format {
        Format::Human => format::human(&pr.head),
        Format::Json => format::json_pr(pr)?,
        Format::Markdown => format::markdown_pr(pr),
        Format::Sarif => format::sarif(&pr.head)?,
    })
}

fn emit(rendered: String, output: Option<&std::path::Path>) -> Result<()> {
    match output {
        Some(path) => {
            std::fs::write(path, rendered)
                .with_context(|| format!("writing output to {}", path.display()))?;
        }
        None => print!("{rendered}"),
    }
    Ok(())
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

fn exit_for_pr(pr: &PrReport, fail_on: FailOn) -> ExitCode {
    let counts = &pr.delta.new_counts;
    let bad = match fail_on {
        FailOn::Never => return ExitCode::from(0),
        FailOn::Warn => counts.error > 0 || counts.warn > 0,
        FailOn::Error => counts.error > 0,
    };
    let verdict_blocks = matches!(pr.verdict, Verdict::RequestChanges { .. });
    if bad || verdict_blocks {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

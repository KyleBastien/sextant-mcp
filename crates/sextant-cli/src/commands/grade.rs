use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::ValueEnum;
use globset::GlobSet;
use ignore::WalkBuilder;
use sextant_config::Config;
use sextant_core::{EvalContext, Finding, Report, SourceFile, Verdict, VerdictThresholds};
use sextant_diff::{compute, BaseSpec, DiffSet, HeadSpec};
use sextant_rules::RuleSet;

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

pub fn run(args: GradeArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let config = Config::from_repo_root(&cwd).context("loading config")?;
    let exclude = config
        .paths
        .matcher()
        .context("building path-exclude matcher")?;
    let ruleset = RuleSet::load(&cwd, &config).context("loading rules")?;
    let ctx = EvalContext { repo_root: &cwd };

    let findings = if args.diff {
        let diff = run_diff(
            &cwd,
            args.base.as_deref(),
            args.head.as_deref(),
            args.working_tree,
        )
        .context("computing diff")?;
        let files = source_files_from_diff(&cwd, &diff, &exclude);
        let raw = ruleset.grade_files(&files, &ctx);
        filter_to_diff(raw, &diff)
    } else {
        let targets = if args.paths.is_empty() {
            vec![cwd.clone()]
        } else {
            args.paths.clone()
        };
        let files = collect_source_files(&cwd, &targets, &exclude)?;
        ruleset.grade_files(&files, &ctx)
    };

    let thresholds: VerdictThresholds = (&config.verdict).into();
    let verdict = thresholds.evaluate(&findings);
    let report = Report::build(findings, verdict);

    match args.format {
        Format::Human => print_human(&report),
        Format::Json => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
        }
    }

    Ok(exit_for(&report, args.fail_on))
}

fn run_diff(
    repo_root: &Path,
    base: Option<&str>,
    head: Option<&str>,
    force_working_tree: bool,
) -> Result<DiffSet> {
    let base_spec = match base {
        Some(s) => BaseSpec::Ref(s.to_string()),
        None => BaseSpec::Auto,
    };
    let head_spec = if force_working_tree {
        HeadSpec::WorkingTree
    } else {
        match head {
            Some(s) => HeadSpec::Ref(s.to_string()),
            None => HeadSpec::WorkingTree,
        }
    };
    Ok(compute(repo_root, &base_spec, &head_spec)?)
}

fn source_files_from_diff(repo_root: &Path, diff: &DiffSet, exclude: &GlobSet) -> Vec<SourceFile> {
    let mut out = Vec::new();
    for f in &diff.files {
        if exclude.is_match(&f.path) {
            continue;
        }
        let Some(contents) = f.head_contents.as_ref() else {
            continue;
        };
        out.push(SourceFile::new(repo_root.join(&f.path), contents.clone()));
    }
    out
}

/// Drop any finding whose head-side line range is entirely outside the diff.
/// Findings without a line (file-scoped) are kept iff the file is in the
/// diff at all.
fn filter_to_diff(findings: Vec<Finding>, diff: &DiffSet) -> Vec<Finding> {
    let mut by_path: std::collections::HashMap<PathBuf, &BTreeSet<u32>> =
        std::collections::HashMap::new();
    for f in &diff.files {
        by_path.insert(f.path.clone(), &f.changed_lines);
    }
    findings
        .into_iter()
        .filter(|f| {
            let Some(changed) = by_path.get(&f.path) else {
                return false;
            };
            match (f.line, f.end_line) {
                (None, _) => !changed.is_empty(),
                (Some(start), end) => {
                    let end = end.unwrap_or(start);
                    (start..=end).any(|ln| changed.contains(&ln))
                }
            }
        })
        .collect()
}

fn collect_source_files(
    root: &Path,
    targets: &[PathBuf],
    exclude: &GlobSet,
) -> Result<Vec<SourceFile>> {
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
            let rel = path.strip_prefix(root).unwrap_or(&path);
            if exclude.is_match(rel) {
                continue;
            }
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

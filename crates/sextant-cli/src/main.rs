use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use globset::GlobSet;
use ignore::WalkBuilder;
use sextant_config::Config;
use sextant_core::{EvalContext, Finding, Report, SourceFile, Verdict, VerdictThresholds};
use sextant_diff::{compute, BaseSpec, DiffSet, HeadSpec};
use sextant_rules::{parse_rule_md, RuleSet};

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
    /// Grade files. Defaults to whole-file mode; pass `--diff` to grade
    /// only changed lines against a base ref.
    Grade {
        /// Paths to grade. Ignored when `--diff` is set. Defaults to the
        /// current directory.
        paths: Vec<PathBuf>,
        /// Switch to diff mode: only findings on changed lines are reported.
        #[arg(long)]
        diff: bool,
        /// Base ref. Default: merge-base with origin/main, falling back to HEAD~1.
        #[arg(long)]
        base: Option<String>,
        /// Head ref. Default: working tree (with index applied).
        #[arg(long)]
        head: Option<String>,
        /// Force diff against the working tree even when --head is set.
        #[arg(long, conflicts_with = "head")]
        working_tree: bool,
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
    /// Print the full markdown documentation for a rule.
    Explain {
        /// Rule id (e.g. `builtin.size.fn-length`).
        id: String,
    },
    /// Validate a rule markdown file's frontmatter without loading it.
    Check {
        /// Path to a `.md` rule file.
        path: PathBuf,
    },
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
            diff,
            base,
            head,
            working_tree,
            format,
            fail_on,
        } => cmd_grade(GradeArgs {
            paths,
            diff,
            base,
            head,
            working_tree,
            format,
            fail_on,
        }),
        Cmd::Rules { cmd } => match cmd {
            RulesCmd::List => cmd_rules_list(),
            RulesCmd::Explain { id } => cmd_rules_explain(&id),
            RulesCmd::Check { path } => cmd_rules_check(&path),
        },
    }
}

struct GradeArgs {
    paths: Vec<PathBuf>,
    diff: bool,
    base: Option<String>,
    head: Option<String>,
    working_tree: bool,
    format: Format,
    fail_on: FailOn,
}

fn cmd_grade(args: GradeArgs) -> Result<ExitCode> {
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

fn cmd_rules_list() -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let config = Config::from_repo_root(&cwd)?;
    let ruleset = RuleSet::load(&cwd, &config).context("loading rules")?;
    for ev in ruleset.evaluators() {
        let r = ev.rule();
        println!(
            "{}\t{}\t{}\t{}\t{}",
            r.id,
            r.severity.as_str(),
            format!("{:?}", r.scope).to_lowercase(),
            r.source.as_str(),
            r.name
        );
    }
    Ok(ExitCode::from(0))
}

fn cmd_rules_explain(id: &str) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let config = Config::from_repo_root(&cwd)?;
    let ruleset = RuleSet::load(&cwd, &config).context("loading rules")?;
    let Some(rule) = ruleset.evaluators().iter().find(|e| e.rule().id == id) else {
        eprintln!("error: no rule with id `{id}`");
        return Ok(ExitCode::from(2));
    };
    let r = rule.rule();
    println!("# {} ({})", r.name, r.id);
    println!();
    println!(
        "**severity:** {}  •  **category:** {}  •  **source:** {}",
        r.severity.as_str(),
        category_str(&r.category),
        r.source.as_str()
    );
    println!();
    if !r.description.is_empty() {
        println!("{}", r.description);
        println!();
    }
    if !r.body.is_empty() {
        println!("{}", r.body);
    }
    Ok(ExitCode::from(0))
}

fn cmd_rules_check(path: &Path) -> Result<ExitCode> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    match parse_rule_md(
        &text,
        sextant_core::RuleSource::Repo,
        Some(path.to_path_buf()),
    ) {
        Ok(rule) => {
            println!("OK: {} ({})", rule.id, rule.name);
            println!(
                "  severity={} category={} scope={:?}",
                rule.severity.as_str(),
                category_str(&rule.category),
                rule.scope,
            );
            match &rule.evaluator {
                sextant_rules::EvaluatorSpec::Builtin { name } => {
                    println!("  evaluator=builtin name={name}");
                }
                sextant_rules::EvaluatorSpec::Regex { pattern, .. } => {
                    println!("  evaluator=regex pattern={pattern:?}");
                }
            }
            Ok(ExitCode::from(0))
        }
        Err(err) => {
            eprintln!("error: {err}");
            Ok(ExitCode::from(2))
        }
    }
}

fn category_str(c: &sextant_core::Category) -> String {
    use sextant_core::Category::*;
    match c {
        Complexity => "complexity".into(),
        Size => "size".into(),
        Duplication => "duplication".into(),
        Tests => "tests".into(),
        Reliability => "reliability".into(),
        Style => "style".into(),
        Security => "security".into(),
        Docs => "docs".into(),
        Custom(s) => format!("custom:{s}"),
    }
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
/// Findings without a line (e.g. file-scoped) are kept iff the file is in
/// the diff at all.
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

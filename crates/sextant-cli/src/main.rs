mod commands;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::grade::{FailOn, Format, GradeArgs};

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
    /// only changed lines against a base ref, or `--pr` for PR-mode
    /// regression grading with a baseline cache.
    Grade {
        /// Paths to grade. Ignored when `--diff` is set. Defaults to the
        /// current directory.
        paths: Vec<PathBuf>,
        /// Switch to diff mode: only findings on changed lines are reported.
        #[arg(long)]
        diff: bool,
        /// PR mode: diff-grade head against a baseline-graded base SHA
        /// and report only *new* findings introduced by the change.
        /// Implies `--diff`.
        #[arg(long, conflicts_with = "diff")]
        pr: bool,
        /// Directory to read/write per-base-SHA baseline reports. Used
        /// by the GitHub Action via `actions/cache` to avoid recomputing
        /// the baseline on every PR run.
        #[arg(long, value_name = "DIR")]
        baseline_cache: Option<PathBuf>,
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
        /// Write the rendered output to PATH instead of stdout. Useful
        /// for piping a markdown review into `gh api ... -F body=@...`.
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Side-channel: also dump the structured report (PR mode:
        /// `PrReport`, otherwise `Report`) as JSON to PATH. Independent
        /// of `--format`, so the GitHub Action can take markdown for the
        /// review while still parsing fields out of JSON.
        #[arg(long, value_name = "PATH")]
        report_json: Option<PathBuf>,
        /// Severity at which to exit non-zero.
        #[arg(long, value_enum, default_value_t = FailOn::Error)]
        fail_on: FailOn,
        /// Skip LLM-evaluated rules (drop them at load time). Useful for
        /// CI runs that should never touch the network.
        #[arg(long)]
        no_llm: bool,
    },
    /// Rule introspection commands.
    Rules {
        #[command(subcommand)]
        cmd: RulesCmd,
    },
    /// Write a `.sextant/` directory with a config and sample rules.
    /// Idempotent: skips files that already exist unless `--force` is
    /// passed.
    Init {
        /// Which scaffold to drop. `default` is language-agnostic;
        /// language-specific templates add a relevant sample rule;
        /// `strict` uses tighter thresholds.
        #[arg(long, value_enum, default_value_t = commands::init::Template::Default)]
        template: commands::init::Template,
        /// Overwrite existing files instead of skipping them.
        #[arg(long)]
        force: bool,
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
    /// Install a vendor rule pack from GitHub or a local path.
    Add {
        /// Pack spec: `github:owner/repo@<ref>[#subdir]` or `file:<path>[#subdir]`.
        spec: String,
        /// Override the pack name (defaults to the value in `pack.toml`).
        #[arg(long)]
        name: Option<String>,
    },
    /// Refresh installed vendor packs from their pinned references.
    Update {
        /// Specific packs to update (empty = all).
        packs: Vec<String>,
    },
    /// Remove an installed vendor pack and drop its lock entry.
    Remove {
        /// Pack name as recorded in `.sextant/rules.lock`.
        name: String,
    },
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
    match dispatch(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(cli: Cli) -> Result<ExitCode> {
    match cli.cmd {
        Cmd::Grade {
            paths,
            diff,
            pr,
            baseline_cache,
            base,
            head,
            working_tree,
            format,
            output,
            report_json,
            fail_on,
            no_llm,
        } => commands::grade::run(GradeArgs {
            paths,
            diff,
            pr,
            baseline_cache,
            base,
            head,
            working_tree,
            format,
            output,
            report_json,
            fail_on,
            no_llm,
        }),
        Cmd::Rules { cmd } => dispatch_rules(cmd),
        Cmd::Init { template, force } => commands::init::run(template, force),
    }
}

fn dispatch_rules(cmd: RulesCmd) -> Result<ExitCode> {
    match cmd {
        RulesCmd::List => commands::rules::list(),
        RulesCmd::Explain { id } => commands::rules::explain(&id),
        RulesCmd::Check { path } => commands::rules::check(&path),
        RulesCmd::Add { spec, name } => commands::rules::add(&spec, name.as_deref()),
        RulesCmd::Update { packs } => commands::rules::update(&packs),
        RulesCmd::Remove { name } => commands::rules::remove(&name),
    }
}

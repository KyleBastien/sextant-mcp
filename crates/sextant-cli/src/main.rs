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
            base,
            head,
            working_tree,
            format,
            fail_on,
        } => commands::grade::run(GradeArgs {
            paths,
            diff,
            base,
            head,
            working_tree,
            format,
            fail_on,
        }),
        Cmd::Rules { cmd } => match cmd {
            RulesCmd::List => commands::rules::list(),
            RulesCmd::Explain { id } => commands::rules::explain(&id),
            RulesCmd::Check { path } => commands::rules::check(&path),
        },
    }
}

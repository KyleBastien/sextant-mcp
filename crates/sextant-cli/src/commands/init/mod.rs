//! `sextant init` — write `.sextant/config.toml` + a sample rule so a
//! fresh repo is one command away from being graded.
//!
//! `--template <name>` picks which scaffold to drop:
//! * `default` — language-agnostic, sensible thresholds, one TODO rule.
//! * `rust` — adds an "no .unwrap() in prod" rule.
//! * `python` — adds a `print()`-in-prod rule.
//! * `go` — adds a `fmt.Println` rule.
//! * `java` — adds a `System.out.println` rule.
//! * `typescript` / `javascript` — add a `console.log`-in-prod rule.
//! * `strict` — same shape as default but tighter thresholds (10/30 for
//!   fn-length, 5 for param-count, etc.) for teams that want to push
//!   back on growth.
//!
//! Refuses to overwrite existing files unless `--force` is passed.
//! Mirrors the `git init` UX: idempotent on a clean repo, loud on a
//! dirty one.

mod templates;

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::ValueEnum;

use templates::{
    DEFAULT_CONFIG, NO_CONSOLE_LOG_RULE, NO_PRINT_GO_RULE, NO_PRINT_JAVA_RULE,
    NO_PRINT_PYTHON_RULE, NO_TODO_RULE, NO_UNWRAP_RULE, STRICT_CONFIG,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Template {
    Default,
    Rust,
    Python,
    Go,
    Java,
    TypeScript,
    JavaScript,
    Strict,
}

struct Scaffold {
    config: &'static str,
    /// `(filename, contents)` for each rule under `.sextant/rules/`.
    rules: &'static [(&'static str, &'static str)],
}

fn scaffold(template: Template) -> Scaffold {
    match template {
        Template::Default => Scaffold {
            config: DEFAULT_CONFIG,
            rules: &[NO_TODO_RULE],
        },
        Template::Rust => Scaffold {
            config: DEFAULT_CONFIG,
            rules: &[NO_TODO_RULE, NO_UNWRAP_RULE],
        },
        Template::Python => Scaffold {
            config: DEFAULT_CONFIG,
            rules: &[NO_TODO_RULE, NO_PRINT_PYTHON_RULE],
        },
        Template::Go => Scaffold {
            config: DEFAULT_CONFIG,
            rules: &[NO_TODO_RULE, NO_PRINT_GO_RULE],
        },
        Template::Java => Scaffold {
            config: DEFAULT_CONFIG,
            rules: &[NO_TODO_RULE, NO_PRINT_JAVA_RULE],
        },
        Template::TypeScript | Template::JavaScript => Scaffold {
            config: DEFAULT_CONFIG,
            rules: &[NO_TODO_RULE, NO_CONSOLE_LOG_RULE],
        },
        Template::Strict => Scaffold {
            config: STRICT_CONFIG,
            rules: &[NO_TODO_RULE],
        },
    }
}

pub(crate) fn run(template: Template, force: bool) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let dir = cwd.join(".sextant");
    let rules_dir = dir.join("rules");
    std::fs::create_dir_all(&rules_dir)
        .with_context(|| format!("creating {}", rules_dir.display()))?;

    let s = scaffold(template);
    let config_path = dir.join("config.toml");
    write_if_absent(&config_path, s.config, force)?;
    for (filename, contents) in s.rules {
        let path = rules_dir.join(filename);
        write_if_absent(&path, contents, force)?;
    }

    println!(
        "Initialized .sextant/ in {} ({:?} template).",
        cwd.display(),
        template
    );
    println!("  config: .sextant/config.toml");
    for (filename, _) in s.rules {
        println!("  rule:   .sextant/rules/{filename}");
    }
    println!();
    println!("Try:  sextant grade  |  sextant rules list  |  sextant rules explain <id>");
    Ok(ExitCode::from(0))
}

fn write_if_absent(path: &Path, contents: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        eprintln!(
            "note: {} exists; skipping (pass --force to overwrite)",
            path.display()
        );
        return Ok(());
    }
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn write_if_absent_skips_existing_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, "original").unwrap();
        write_if_absent(&path, "replacement", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn write_if_absent_overwrites_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, "original").unwrap();
        write_if_absent(&path, "replacement", true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "replacement");
    }

    #[test]
    fn write_if_absent_creates_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.txt");
        write_if_absent(&path, "hello", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn rust_template_includes_unwrap_rule() {
        let s = scaffold(Template::Rust);
        let names: Vec<_> = s.rules.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"no-unwrap.md"), "got: {names:?}");
    }

    #[test]
    fn typescript_template_includes_console_log_rule() {
        let s = scaffold(Template::TypeScript);
        let names: Vec<_> = s.rules.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"no-console-log.md"), "got: {names:?}");
    }

    #[test]
    fn strict_template_uses_tighter_thresholds() {
        let s = scaffold(Template::Strict);
        assert!(s.config.contains("fn_length_warn = 30"));
        assert!(s.config.contains("min_tokens = 60"));
    }

    #[test]
    fn every_template_emits_at_least_one_rule() {
        for t in [
            Template::Default,
            Template::Rust,
            Template::Python,
            Template::Go,
            Template::Java,
            Template::TypeScript,
            Template::JavaScript,
            Template::Strict,
        ] {
            assert!(!scaffold(t).rules.is_empty(), "{t:?} has no rules");
        }
    }
}

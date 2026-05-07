//! `sextant init` — write `.sextant/config.toml` + a sample rule so a
//! fresh repo is one command away from being graded.
//!
//! Refuses to overwrite existing files unless `--force` is passed.
//! Mirrors the `git init` UX: idempotent on a clean repo, loud on a
//! dirty one.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

const CONFIG_TEMPLATE: &str = r#"# Sextant configuration. See https://github.com/kylebastien/sextant-mcp
# for the full schema. Every section is optional — defaults are sensible.

[verdict]
# `absolute` (default) counts every finding; `regression` only flags
# new findings vs the baseline. `--pr` always runs in regression mode.
mode = "absolute"
max_errors = 0
max_warns = 100

[size]
file_length_warn = 400
file_length_error = 800
fn_length_warn = 60
fn_length_error = 120
param_count_warn = 6
param_count_error = 10

[complexity]
cyclomatic_warn = 10
cyclomatic_error = 20
nesting_warn = 4
nesting_error = 6

[duplication]
min_tokens = 100

# Uncomment to enable LLM-as-judge rules. The api key lives in an env
# var so it's not checked in.
# [judge]
# enabled = true
# provider = "anthropic"        # or "openai" | "openai-compatible"
# model = "claude-sonnet-4-6"
# max_tokens = 1024
# api_key_env = "ANTHROPIC_API_KEY"
"#;

const SAMPLE_RULE: &str = r#"---
id: project.no-todo
name: "No TODO comments"
description: "Avoid shipping TODO markers in production code."
severity: warn
category: style
scope: file
languages: [rust, python, typescript, javascript]
evaluator:
  type: regex
  pattern: "TODO"
  exclude_paths: ["**/tests/**", "**/*_test.*"]
enabled: true
tags: [style]
---

# No TODO comments

Flags any line containing the word `TODO` outside of test files.

## Why

TODOs accumulate. Track work in your issue tracker instead, where it's
visible to the team and gets prioritized like everything else.

## Fixing

- Move the TODO into an issue and link the issue id in a comment.
- Or: do the work now, since you're already in this code.
"#;

pub(crate) fn run(force: bool) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let dir = cwd.join(".sextant");
    let rules_dir = dir.join("rules");
    std::fs::create_dir_all(&rules_dir)
        .with_context(|| format!("creating {}", rules_dir.display()))?;

    let config_path = dir.join("config.toml");
    let rule_path = rules_dir.join("no-todo.md");

    write_if_absent(&config_path, CONFIG_TEMPLATE, force)?;
    write_if_absent(&rule_path, SAMPLE_RULE, force)?;

    println!("Initialized .sextant/ in {}.", cwd.display());
    println!("  config: .sextant/config.toml");
    println!("  rule:   .sextant/rules/no-todo.md");
    println!();
    println!(
        "Try:  sextant grade  |  sextant rules list  |  sextant rules explain project.no-todo"
    );
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
}

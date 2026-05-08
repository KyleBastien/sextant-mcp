//! `sextant init` — write `.sextant/config.toml` + a sample rule so a
//! fresh repo is one command away from being graded.
//!
//! `--template <name>` picks which scaffold to drop:
//! * `default` — language-agnostic, sensible thresholds, one TODO rule.
//! * `rust` — adds an "no .unwrap() in prod" rule and pins
//!   `languages: [rust]` on the sample.
//! * `python` — adds a `print()`-in-prod rule.
//! * `go` — adds a `fmt.Println` rule.
//! * `java` — adds a `System.out.println` rule.
//! * `strict` — same shape as default but tighter thresholds (10/30 for
//!   fn-length, 5 for param-count, etc.) for teams that want to push
//!   back on growth.
//!
//! Refuses to overwrite existing files unless `--force` is passed.
//! Mirrors the `git init` UX: idempotent on a clean repo, loud on a
//! dirty one.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Template {
    Default,
    Rust,
    Python,
    Go,
    Java,
    Strict,
}

struct Scaffold {
    config: &'static str,
    /// `(filename, contents)` for each rule under `.sextant/rules/`.
    rules: &'static [(&'static str, &'static str)],
}

const DEFAULT_CONFIG: &str = r#"# Sextant configuration. See https://github.com/kylebastien/sextant-mcp
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

const STRICT_CONFIG: &str = r#"# Sextant configuration — strict variant. Tighter thresholds; pushes
# back harder on growth. Calibrate downwards if your codebase needs more
# room than this.

[verdict]
mode = "absolute"
max_errors = 0
max_warns = 25

[size]
file_length_warn = 250
file_length_error = 500
fn_length_warn = 30
fn_length_error = 80
param_count_warn = 4
param_count_error = 6

[complexity]
cyclomatic_warn = 7
cyclomatic_error = 12
nesting_warn = 3
nesting_error = 5

[duplication]
min_tokens = 60
"#;

const NO_TODO_RULE: (&str, &str) = (
    "no-todo.md",
    r#"---
id: project.no-todo
name: "No TODO comments"
description: "Avoid shipping TODO markers in production code."
severity: warn
category: style
scope: file
languages: [rust, python, typescript, javascript, go, java]
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
"#,
);

const NO_UNWRAP_RULE: (&str, &str) = (
    "no-unwrap.md",
    r#"---
id: project.no-unwrap
name: "No unwrap() in prod code"
description: "`.unwrap()` and `.expect()` panic on Err/None — bad in prod paths."
severity: warn
category: reliability
scope: file
languages: [rust]
evaluator:
  type: regex
  pattern: '\.(unwrap|expect)\s*\('
  exclude_paths: ["**/tests/**", "**/*_test.rs", "**/build.rs", "**/examples/**"]
enabled: true
tags: [rust, panics]
---

# No unwrap() in prod

Match any `.unwrap()` or `.expect(...)` outside tests, build scripts,
and examples. These panic on `Err`/`None` — fine for tests, dangerous
for code on a request path.

## Fixing

- Use `?` to propagate the error and let the caller decide.
- Use `.unwrap_or(default)` / `.unwrap_or_else(|| ...)` for a fallback.
- If a None *truly* can't happen, leave a one-line `expect("invariant: ...")`
  with a real reason — silences the rule via comment context.
"#,
);

const NO_PRINT_PYTHON_RULE: (&str, &str) = (
    "no-print.md",
    r#"---
id: project.no-print
name: "No bare print() in prod"
description: "Use logging instead of bare prints."
severity: warn
category: style
scope: file
languages: [python]
evaluator:
  type: regex
  pattern: '^\s*print\s*\('
  exclude_paths: ["**/tests/**", "**/scripts/**", "**/examples/**"]
enabled: true
tags: [python, logging]
---

# No bare print()

Bare `print(...)` calls in production code lose level, structure, and
destination control. Use the project's logger.

## Fixing

- `import logging` once at module top, then `logging.info(...)`,
  `logging.warning(...)`, etc. Match the level to the actual severity.
- For CLI tools that print results to stdout, suppress this rule on
  the entry-point file by adding it to `exclude_paths` in your repo
  override of this rule.
"#,
);

const NO_PRINT_GO_RULE: (&str, &str) = (
    "no-fmt-println.md",
    r#"---
id: project.no-fmt-println
name: "No fmt.Println in prod"
description: "Use a structured logger (slog, zap, zerolog) instead."
severity: warn
category: style
scope: file
languages: [go]
evaluator:
  type: regex
  pattern: '\bfmt\.(Print|Println|Printf)\s*\('
  exclude_paths: ["**/cmd/**", "**/examples/**", "**/*_test.go", "**/main.go"]
enabled: true
tags: [go, logging]
---

# No fmt.Println in prod

`fmt.Println` and friends bypass log levels and structured-logging
infrastructure. They show up unfiltered in CI and on production stdout.

## Fixing

- Use `slog.Info` / `slog.Error` (Go 1.21+) or your project's logger.
- For CLI tools, the rule already excludes `cmd/`, `examples/`, and
  `main.go`. Extend `exclude_paths` if you have other entry points.
"#,
);

const NO_PRINT_JAVA_RULE: (&str, &str) = (
    "no-system-out.md",
    r#"---
id: project.no-system-out
name: "No System.out in prod"
description: "Use SLF4J / Log4j / java.util.logging instead."
severity: warn
category: style
scope: file
languages: [java]
evaluator:
  type: regex
  pattern: '\bSystem\.(out|err)\.(println|print|printf)\s*\('
  exclude_paths: ["**/test/**", "**/*Test.java", "**/Main.java", "**/examples/**"]
enabled: true
tags: [java, logging]
---

# No System.out in prod

`System.out.println` (and its `System.err` cousin) bypass the logger,
make it impossible to filter by level, and don't carry MDC context.

## Fixing

- Use SLF4J: `private static final Logger log =
  LoggerFactory.getLogger(MyClass.class);`, then `log.info(...)`.
- For one-off `Main`-style entry points, the rule excludes `Main.java`
  by default. Extend `exclude_paths` for other entry points.
"#,
);

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
            Template::Strict,
        ] {
            assert!(!scaffold(t).rules.is_empty(), "{t:?} has no rules");
        }
    }
}

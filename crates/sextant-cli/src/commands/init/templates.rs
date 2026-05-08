//! Template fixtures for `sextant init`. Lifted out of `init/mod.rs` so
//! that file stays under the file-length threshold — the rule bodies
//! are static text, kept verbatim here.

pub(super) const DEFAULT_CONFIG: &str = r#"# Sextant configuration. See https://github.com/kylebastien/sextant-mcp
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

pub(super) const STRICT_CONFIG: &str = r#"# Sextant configuration — strict variant. Tighter thresholds; pushes
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

pub(super) const NO_TODO_RULE: (&str, &str) = (
    "no-todo.md",
    r#"---
id: project.no-todo
name: "No TODO comments"
description: "Avoid shipping TODO markers in production code."
severity: warn
category: style
scope: file
languages: [rust, python, go, java, typescript, tsx, javascript]
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

pub(super) const NO_UNWRAP_RULE: (&str, &str) = (
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

pub(super) const NO_PRINT_PYTHON_RULE: (&str, &str) = (
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

pub(super) const NO_PRINT_GO_RULE: (&str, &str) = (
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

pub(super) const NO_PRINT_JAVA_RULE: (&str, &str) = (
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

pub(super) const NO_CONSOLE_LOG_RULE: (&str, &str) = (
    "no-console-log.md",
    r#"---
id: project.no-console-log
name: "No console.log in prod"
description: "Use a real logger; console.log loses level + structure."
severity: warn
category: style
scope: file
languages: [typescript, tsx, javascript]
evaluator:
  type: regex
  pattern: '\bconsole\.(log|info|warn|error|debug)\s*\('
  exclude_paths: ["**/test/**", "**/tests/**", "**/*.test.*", "**/*.spec.*", "**/scripts/**"]
enabled: true
tags: [typescript, javascript, logging]
---

# No console.log in prod

`console.log` (and its `.info`/`.warn`/`.error`/`.debug` cousins) bypass
log levels and structured-logging infrastructure. They show up
unfiltered in production stdout/stderr and don't carry trace context.

## Fixing

- Use the project's logger — `pino`, `winston`, `bunyan`, the
  `@opentelemetry/*` logger, or whatever the codebase has standardised on.
- For genuine CLI output (where stdout *is* the deliverable), the rule
  already excludes `scripts/`. Extend `exclude_paths` for other
  CLI-style entry points.
- Tests are excluded by default — `**/*.test.*` and `**/*.spec.*`.
"#,
);

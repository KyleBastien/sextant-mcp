# sextant-mcp

Code-quality grader for AI-agent workflows. Two surfaces share one engine:

- **MCP server** — agents call `grade_diff` after each edit and self-correct
  before finishing the turn.
- **GitHub Action / CLI** — runs on every PR, posts a CodeScene-style
  summary review, and gates the check on regression.

Status: M1 — workspace skeleton, core types, and one deterministic rule
(`builtin.size.file-length`) wired through the CLI.

## Quickstart

```sh
cargo build --workspace
cargo run -p sextant-cli -- grade
cargo run -p sextant-cli -- rules list
```

Configure thresholds in `.sextant/config.toml`:

```toml
[verdict]
max_errors = 0
max_warns = 50

[size]
file_length_warn = 400
file_length_error = 800
```

## Layout

```
crates/
  sextant-core/    types: Rule, Finding, Report, Verdict, Evaluator trait
  sextant-config/  TOML config loader
  sextant-rules/   built-in rule evaluators
  sextant-cli/     `sextant` binary
```

## License

MIT OR Apache-2.0

---
title: Per-crate summary
description: One paragraph per crate in the workspace.
sidebar:
  order: 3
---

The
[`crates/`](https://github.com/kylebastien/sextant-mcp/tree/main/crates)
directory in the repo. Linked diagram is in the
[architecture overview](/sextant-mcp/architecture/).

## `sextant-core`

The type definitions every other crate depends on. `Rule`, `Finding`,
`Report`, `Verdict`, `Severity`, `Category`, `Scope`, `EvaluatorSpec`,
the `Evaluator` trait, the `BaselineDelta` and `PrReport` types for
regression mode, plus a few helpers like `SeverityCounts` and
`VerdictThresholds`. **No I/O dependencies.** That guarantee is what
lets the crate stay stable while the surface crates churn — and makes
it cheap to test.

## `sextant-config`

Reads `.sextant/config.toml` into typed structs and merges with
defaults. Returns the resolved config the engine runs with. Also
hosts the `[paths] exclude` glob compiler — every crate that walks
files asks this crate which files to skip.

## `sextant-rules`

Two responsibilities: rule discovery and built-in evaluators.
Discovery walks `.sextant/rules/**/*.md`, parses YAML frontmatter +
markdown body via `gray_matter`, validates against the schema, and
returns `Rule` values. Built-ins are embedded in the binary via
`rust-embed` and parsed the same way — they're the seven shipped
rules, plus their Rust evaluator implementations.

## `sextant-diff`

Git diff acquisition via `git2`. Resolves base refs (merge-base with
`origin/main`, falling back to `HEAD~1`). Walks blob contents — no
`git checkout` required, so the working tree stays at the head
commit. Returns the changed files and per-file line ranges that
diff-mode and PR-mode grading need.

## `sextant-lang`

Tree-sitter parsers and queries for Rust, Python, Go, Java,
TypeScript, TSX, JavaScript. Provides cached parsers (one per
language, reused across files) and the captured-name queries built-in
evaluators use to find functions, types, etc. Adding a language is
isolated to this crate.

## `sextant-judge`

LLM-as-judge providers and cache. Wraps Anthropic and OpenAI HTTP
APIs (via `reqwest`), enforces tool-use schemas so LLM responses are
well-typed `Finding`s, and caches by BLAKE3 hash of `(file content,
rule id, rule body, model)`. The cache makes repeat grades of
unchanged files free.

## `sextant-engine`

Grading orchestration. Public API:

- `grade(cwd, GradeMode) -> Report`
- `grade_pr(cwd, DiffOptions, PrOptions) -> PrReport`
- `list_rules(cwd) -> Vec<RuleSummary>`
- `explain_rule(cwd, id) -> Option<RuleSummary>`
- `load_config(cwd) -> Config`

That's the whole engine API. Both binaries call into exactly these
functions; the rest is wire-format wrapping. The engine's job is to
load config + rules, acquire files, run evaluators, build a report —
no command-line parsing, no JSON-RPC framing, no markdown rendering.

## `sextant-cli`

The `sextant` binary. Wraps the engine with `clap` argument parsing
and one render module per output format (`human`, `json`, `markdown`,
`sarif`, `review-json`). Subcommands: `grade`, `rules` (with
`list` / `explain` / `check`), and `init`. Tests are largely
snapshot-based via `insta`.

## `sextant-mcp`

The `sextant-mcp` binary. JSON-RPC 2.0 server — stdio by default,
HTTP via `axum` when `--http <addr>` is passed. Five tools:
`grade_diff`, `grade_files`, `list_rules`, `explain_rule`,
`get_config`. Each tool dispatches to one engine entry point and
wraps the result in the MCP `content` envelope. Logging goes to
stderr; stdout is reserved for the protocol stream.

## See also

- [Architecture overview](/sextant-mcp/architecture/) — dependency
  graph and motivation.
- [Data model](/sextant-mcp/architecture/data-model/) — types
  exchanged between crates.
- [`Cargo.toml`](https://github.com/kylebastien/sextant-mcp/blob/main/Cargo.toml) —
  the canonical workspace definition.

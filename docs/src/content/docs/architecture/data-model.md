---
title: Data model
description: How Rule, Finding, Report, and Verdict flow through a grade.
sidebar:
  order: 2
---

The four core types form a tight chain. Every type lives in
`sextant-core` and has no I/O — they're pure data, derived everywhere
they appear (CLI JSON, MCP responses, GitHub PR comments, SARIF).

## The chain

```
.sextant/config.toml ─┐
                      ├─► Rule  ──┐
.sextant/rules/*.md  ─┘           │
                                  ├─► Finding[]  ──► Report ──► Verdict
files in scope (diff/whole) ─────►┘
```

| Step | Input | Output | Crate |
|---|---|---|---|
| Load rules | Config + rule markdown | `Rule[]` | `sextant-rules` |
| Acquire files | Git refs / paths | file set | `sextant-diff` / engine |
| Parse | files | ASTs (when needed) | `sextant-lang` |
| Evaluate | `Rule + file` | `Finding[]` | `sextant-rules` / `sextant-judge` |
| Aggregate | `Finding[]` | `Report` | `sextant-engine` (via `sextant-core`) |
| Verdict | `Report.counts` + `[verdict]` thresholds | `Verdict` | `sextant-core` |

## Type sketches

Sketches — see the linked concept pages for the full schemas.

```rust
// sextant-core/src/rule.rs
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: Severity,
    pub category: Category,
    pub scope: Scope,
    pub languages: Vec<String>,
    pub source: RuleSource,
    pub enabled: bool,
    pub overrides: Vec<String>,
    pub tags: Vec<String>,
    pub body: String,
    pub evaluator: EvaluatorSpec,
}

// sextant-core/src/finding.rs
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub path: PathBuf,
    pub line: Option<u32>,
    pub end_line: Option<u32>,
}

// sextant-core/src/report.rs
pub struct Report {
    pub findings: Vec<Finding>,
    pub counts: SeverityCounts,
    pub verdict: Verdict,
    pub summary: String,
}

// sextant-core/src/verdict.rs
pub enum Verdict {
    Approve,
    RequestChanges { reasons: Vec<String> },
}
```

## Why no `Rule` in the report

`Report` carries `Finding`s, not the rules that produced them. The
rules are stable across a grade, the findings are the variable thing
the report describes. Callers wanting rule metadata fetch it
separately via `list_rules` / `explain_rule`.

This also keeps reports small. A repo with 50 rules and 3 findings
serializes to a few KB of JSON, not several pages of rule metadata.

## PrReport — regression layer

PR mode (`sextant grade --pr`) wraps the same primitive types in a
delta wrapper:

```rust
pub struct PrReport {
    pub head: Report,
    pub baseline: Report,
    pub delta: BaselineDelta,
    pub verdict: Verdict,
    pub summary: String,
}

pub struct BaselineDelta {
    pub new: Vec<Finding>,
    pub fixed: Vec<Finding>,
    pub unchanged_count: u32,
    pub new_counts: SeverityCounts,
    pub fixed_counts: SeverityCounts,
}
```

`PrReport.verdict` is computed against `delta.new_counts`, not
against `head.counts` — that's where the "regression mode" semantics
come from.

## Determinism

`sextant-core` sorts `findings` by `(severity desc, path, line)` when
building a `Report`. That ordering is preserved through every output
format, so two grades of the same repo with the same rules and same
files produce byte-identical JSON. The cache (LLM responses, baseline
reports) relies on this — a finding's identity is its tuple, not a
random id.

## See also

- [Rule](/sextant-mcp/concepts/rule/),
  [Finding](/sextant-mcp/concepts/finding/),
  [Report](/sextant-mcp/concepts/report/),
  [Verdict](/sextant-mcp/concepts/verdict/) — full concept pages.
- [Architecture overview](/sextant-mcp/architecture/) — crate
  dependency graph.

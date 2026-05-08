---
title: Finding
description: A single match of a rule against a file at a specific line range.
sidebar:
  order: 3
---

A **finding** is one rule firing against one piece of code. A grade
returns a list of findings; the verdict is computed from their severities.

## Shape

```json
{
  "rule_id": "builtin.size.fn-length",
  "severity": "warn",
  "message": "Function `dispatch` is 78 lines (warn at 60)",
  "path": "src/handlers.rs",
  "line": 88,
  "end_line": 165
}
```

| Field | Type | Notes |
|---|---|---|
| `rule_id` | string | The `id` of the rule that fired. Pass to `explain_rule` for full context. |
| `severity` | `info` \| `warn` \| `error` | See [Severity](#severity) below. |
| `message` | string | Human-facing description of the violation, often with a fix hint. |
| `path` | string | File path relative to the repo root. |
| `line` | number? | First line of the violation. Optional — file-scope findings may omit it. |
| `end_line` | number? | Last line of the violation, inclusive. Optional. |

## Severity

Severities are ordered: `info < warn < error`.

| Severity | Meaning |
|---|---|
| `info` | Informational signal. Never blocks a verdict. Use for "good to know" findings. |
| `warn` | Advisory. Counts toward `[verdict] max_warns`. Default thresholds let warns through. |
| `error` | Blocks the default verdict (`max_errors = 0`). Use sparingly — for rules you'd block a PR over. |

The default `[verdict]` thresholds are `max_errors = 0` and
`max_warns = u32::MAX`. The agent surfaces all of them; CI gates only on
errors. Tune to taste in
[`.sextant/config.toml`](/sextant-mcp/configuration/verdict/).

## Sort order

Findings within a report are deterministically sorted by:

1. Severity, descending — errors first.
2. Path, ascending.
3. Line, ascending.

That ordering is preserved across all output formats (JSON, markdown,
SARIF) so diffs between reports are stable.

## In diff and PR mode

Diff-mode grades attach a finding only when its line range overlaps with
the diff. PR-mode grades go further: they compare findings to a
baseline-graded base SHA and report just the **new** ones — see
[Scopes](/sextant-mcp/concepts/scopes/) and
[Verdict → regression mode](/sextant-mcp/concepts/verdict/#regression-mode).

## See also

- [Report](/sextant-mcp/concepts/report/) — the bundle of findings.
- [Verdict](/sextant-mcp/concepts/verdict/) — how findings translate to
  approve/request_changes.

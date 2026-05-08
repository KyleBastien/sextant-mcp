---
title: Verdict
description: How findings translate to approve or request_changes.
sidebar:
  order: 5
---

A **verdict** is the binary output of a grade: either `approve` or
`request_changes` (with reasons). It's computed deterministically from a
report's findings against a configured set of thresholds.

## Shape

```json
{ "kind": "approve" }
```

```json
{
  "kind": "request_changes",
  "reasons": [
    "1 error finding (max allowed: 0)",
    "63 warn findings (max allowed: 50)"
  ]
}
```

The verdict is part of every [Report](/sextant-mcp/concepts/report/).

## Thresholds

Verdicts are derived from `[verdict]` in `.sextant/config.toml`:

```toml
[verdict]
max_errors = 0     # any errors → request_changes
max_warns = 50     # > 50 warns → request_changes (default: u32::MAX)
```

| Field | Default | Effect |
|---|---|---|
| `max_errors` | `0` | Findings with `severity: error` over this count flip the verdict. |
| `max_warns` | `u32::MAX` | Same, for warns. The default is "never block on warns". |

`info` findings never affect the verdict.

## Modes

A verdict has two modes — controlled by which command produced the
report:

### Absolute mode

Used by:

- `sextant grade` (default whole-file mode)
- `sextant grade --diff`
- The `grade_diff` and `grade_files` MCP tools

Counts every finding against thresholds. Whatever's in the report is
what's measured.

### Regression mode

Used by:

- `sextant grade --pr`
- The GitHub Action

Counts only findings that are **new** relative to a baseline-graded base
SHA. A PR that exposes pre-existing problems but doesn't introduce new
ones approves.

This is why the Action runs `sextant grade --pr`: it gives "this PR
makes the codebase worse" semantics rather than "this codebase has
debt" semantics. Pre-existing debt is its own problem.

## Reasons

When the verdict is `request_changes`, `reasons` lists the specific
threshold checks that failed. Both PR mode and absolute mode populate
this — agents and review comments can quote it directly.

## Default behaviour

Out of the box (`max_errors = 0`, no `max_warns`), Sextant blocks only
on errors. Warns surface in the report and the agent's context but
don't gate.

To make warns blocking, set `max_warns` in your config:

```toml
[verdict]
max_errors = 0
max_warns = 0   # any new warn blocks the verdict
```

## See also

- [Configuration → verdict](/sextant-mcp/configuration/verdict/) —
  threshold tuning.
- [Finding → severity](/sextant-mcp/concepts/finding/#severity) — which
  count goes where.
- [Report](/sextant-mcp/concepts/report/) — where the verdict lives.

---
title: Report
description: The output of a grade — findings, severity counts, verdict, summary.
sidebar:
  order: 4
---

A **report** is what every grade returns. It bundles findings with
top-level metadata so a caller (agent, CLI, Action) can render a
verdict without recomputing anything.

## Shape

```json
{
  "summary": "12 findings (0 errors, 7 warns, 5 info). Verdict: approve.",
  "verdict": { "kind": "approve" },
  "counts": { "info": 5, "warn": 7, "error": 0 },
  "findings": [
    {
      "rule_id": "builtin.size.file-length",
      "severity": "warn",
      "path": "src/parser.rs",
      "line": 412,
      "message": "File length 412 exceeds warn threshold (400)"
    },
    …
  ]
}
```

| Field | Type | Notes |
|---|---|---|
| `summary` | string | One-line synopsis suitable for an agent or a chat reply. Read this first. |
| `verdict` | object | `{ "kind": "approve" }` or `{ "kind": "request_changes", "reasons": [...] }`. See [Verdict](/sextant-mcp/concepts/verdict/). |
| `counts` | object | `{ info, warn, error }` totals. |
| `findings` | array | All findings, sorted by `(severity desc, path, line)`. See [Finding](/sextant-mcp/concepts/finding/). |

## PrReport (PR mode)

`sextant grade --pr` returns a richer wrapper around two reports:

```json
{
  "head": { /* full Report for the PR head */ },
  "baseline": { /* full Report for the base SHA */ },
  "delta": {
    "new": [ /* findings only in head */ ],
    "fixed": [ /* findings only in baseline */ ],
    "unchanged_count": 12,
    "new_counts": { "info": 0, "warn": 1, "error": 0 },
    "fixed_counts": { "info": 0, "warn": 2, "error": 0 }
  },
  "verdict": { "kind": "approve" },
  "summary": "1 new finding (1 warn). 2 findings fixed. Verdict: approve."
}
```

The verdict on a `PrReport` is computed against `delta.new_counts`, not
against the head report's totals — a PR that merely *exposes* an
existing finding doesn't block. See
[Verdict → regression mode](/sextant-mcp/concepts/verdict/#regression-mode).

## Output formats

| Format | What you get |
|---|---|
| `human` | Coloured terminal output for `sextant grade` (default). |
| `json` | The structured `Report` (or `PrReport`) above. |
| `markdown` | A PR-comment-friendly markdown rendering. **PR mode only.** |
| `sarif` | SARIF 2.1.0 for GitHub Code Scanning. |
| `review-json` | A `Review` payload ready to POST to the GitHub PR Reviews API. **PR mode only.** |

The `--report-json <PATH>` CLI flag is a side-channel: it always writes
the structured report to a file, even when `--format` is markdown or
SARIF. The Action uses this to render a markdown review while still
parsing the JSON for verdict / counts.

## See also

- [Finding](/sextant-mcp/concepts/finding/) — what's inside `findings`.
- [Verdict](/sextant-mcp/concepts/verdict/) — how `verdict` is derived.
- [Output formats](/sextant-mcp/cli/grade/#output-formats) — picking
  between them on the CLI.

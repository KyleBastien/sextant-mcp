---
title: Concepts
description: The data model behind Sextant — Rule, Finding, Report, Verdict, Evaluator, and Scope.
sidebar:
  order: 1
---

Sextant has a small, deliberately flat data model. Everything you'll
encounter in JSON output, MCP responses, and PR comments comes from these
six types:

| Concept | What it is |
|---|---|
| [**Rule**](/sextant-mcp/concepts/rule/) | A single check, identified by a dotted id (e.g. `builtin.size.fn-length`). |
| [**Finding**](/sextant-mcp/concepts/finding/) | One match of a rule against a file, at a specific line range. |
| [**Report**](/sextant-mcp/concepts/report/) | The output of a grade — findings, severity counts, verdict, summary. |
| [**Verdict**](/sextant-mcp/concepts/verdict/) | `approve` or `request_changes`, derived from configured thresholds. |
| [**Evaluator**](/sextant-mcp/concepts/evaluator/) | The kind of check a rule performs: `builtin`, `regex`, or `llm`. |
| [**Scope**](/sextant-mcp/concepts/scopes/) | The slice of code a grade covers: `diff`, `file`, or `repo`. |

The types live in the `sextant-core` crate and have no I/O dependencies —
the same definitions back the CLI's JSON, the MCP server's tool results,
and the GitHub Action's review comment.

## Recommended reading order

If you've never used Sextant before, read in this order:

1. [Rule](/sextant-mcp/concepts/rule/) — what gets checked.
2. [Evaluator](/sextant-mcp/concepts/evaluator/) — how it gets checked.
3. [Finding](/sextant-mcp/concepts/finding/) — what comes out.
4. [Report](/sextant-mcp/concepts/report/) — how findings are bundled.
5. [Verdict](/sextant-mcp/concepts/verdict/) — how a report becomes
   approve / request_changes.
6. [Scope](/sextant-mcp/concepts/scopes/) — where in the codebase a
   grade looks.

After that, the [CLI reference](/sextant-mcp/cli/) and
[MCP tool reference](/sextant-mcp/mcp/) read like API docs.

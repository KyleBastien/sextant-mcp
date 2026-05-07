---
description: Grade the current state of the repo (or specific paths) and summarize findings.
argument-hint: "[paths...]"
allowed-tools: ["mcp__sextant__grade_diff", "mcp__sextant__grade_files", "mcp__sextant__explain_rule"]
---

Grade the repo using Sextant. Arguments: `$ARGUMENTS`

If `$ARGUMENTS` is empty, call the `mcp__sextant__grade_diff` tool with
default options (working tree vs merge-base) — that's the cheap
inner-loop grade.

If specific paths were passed, call `mcp__sextant__grade_files` with
`{ paths: <split-on-whitespace> }`.

Then summarize the report:

1. State the verdict (`approve` or `request_changes`) and the counts.
2. List the top three findings: severity, rule id, file:line, message.
3. If any rule id is unfamiliar, look it up via
   `mcp__sextant__explain_rule` and include a one-line description.
4. If verdict is `request_changes`, suggest the most actionable next
   step. Don't apply fixes unless the user asks.

---
name: sextant-grade
description: |
  Use when the user asks for code-quality feedback, a code review, or a
  pre-commit check, or when interpreting Sextant findings/verdicts.
  Covers when to call `grade_diff` vs `grade_files`, how to read the
  report, and what severities mean.
---

# Sextant grading

Sextant is a deterministic + LLM code grader exposed via an MCP server.
Two grading scopes you'll use:

- **`grade_diff`** — only findings on changed lines since `base`.
  Fast (typically <500ms). Call this in the inner edit loop after each
  meaningful change. Defaults: base = merge-base with `origin/main`,
  head = working tree.
- **`grade_files`** — full grade of the listed files (or the repo if
  `paths` is empty). Slower (5s+ on a large tree). Use for thorough
  review, not for tight loops.

When you're not sure, prefer `grade_diff` — it's almost always what the
agent loop wants.

## Reading the report

Each finding has:

- `rule_id` — pass to `explain_rule` if the message isn't enough.
- `severity` — `info`, `warn`, or `error`. Errors block the verdict by
  default; warns are advisory.
- `path` + `line`/`end_line` — where to look.
- `message` — what's wrong, often with a fix suggestion.
- `patch` (optional) — a unified diff against the file at HEAD that
  proposes a concrete fix. When present, prefer applying the patch over
  re-deriving the change from scratch: it's the rule (or the synthesis
  judge) telling you what it expects. Use your normal edit tools to
  apply; Sextant does not modify your working tree itself.

Top-level fields:

- `summary` — a one-line LLM-friendly synopsis. Read this first.
- `verdict` — `approve` or `request_changes` with reasons.
- `counts` — `{error, warn, info}`.

## Acting on findings

1. Read `summary`. If `approve`, you're done.
2. If `request_changes`, walk findings in severity order. Errors first.
3. For unfamiliar rule ids, call `explain_rule` to see the rule's
   markdown body — it usually explains the *why* and the fix pattern.
4. After fixing, call `grade_diff` again. Don't end the turn until the
   verdict flips to `approve` or you've explained why a finding is a
   false positive.

## When NOT to call

- During exploration (you haven't written anything yet).
- For docs-only edits in `.md` files where no rules apply.
- After every keystroke. Wait until you've finished a coherent change
  — `grade_diff` is cheap but not free, and findings on half-written
  code aren't actionable.

---
title: Scopes
description: The slice of code a grade covers — diff, file, or repo.
sidebar:
  order: 7
---

**Scope** appears in two places in Sextant, with related but distinct
meanings:

1. On a [rule](/sextant-mcp/concepts/rule/) — what slice the rule's
   evaluator sees when it runs.
2. On a grade — what slice of the codebase Sextant grades.

## Rule scopes

Set on each rule via the `scope:` frontmatter field.

| Scope | Evaluator sees | Used by |
|---|---|---|
| `diff` | Just the changed lines, with a small ± context window. | Rules that only make sense for new code (e.g., "new `unwrap()`"). |
| `file` | Whole file contents. | Most rules — file length, function length, complexity. |
| `repo` | All files at once. | Cross-file rules (cross-file duplication, untested public APIs). On the roadmap. |

A rule's scope determines which grade modes can fire it: a `repo`-scope
rule can't run in pure diff mode, because the engine doesn't have the
rest of the repo. The engine handles this automatically — rules are
filtered by scope per grade.

## Grade modes

The CLI and the MCP server expose three grade modes. Each grades a
different slice of the codebase.

### Diff mode

```sh
sextant grade --diff
sextant grade --diff --base origin/main --head HEAD
```

- Walks the diff between `base` and `head` (default head: working tree).
- Runs `file`-scope rules against the modified files, but reports a
  finding only if its line range overlaps with the diff.
- Runs `diff`-scope rules against just the diff hunks.
- Skips `repo`-scope rules entirely.

This is the **fast path**. Sub-second on a typical change. Use it in
the inner agent loop and as the default CI mode.

The MCP equivalent is the
[`grade_diff`](/sextant-mcp/mcp/tools/grade-diff/) tool.

### Whole-file mode

```sh
sextant grade
sextant grade src/parser.rs src/lexer.rs
```

- Default mode of the CLI.
- Runs `file`-scope rules against every file (or just the listed paths).
- Skips `diff`-scope rules — there's no diff to fire on.
- Useful for an audit, a baseline-fresh repo, or a focused review of
  specific files.

The MCP equivalent is the
[`grade_files`](/sextant-mcp/mcp/tools/grade-files/) tool.

### PR mode

```sh
sextant grade --pr --base $BASE_SHA --head $HEAD_SHA
```

- Runs a full whole-file grade against `base` (cached) and `head`.
- Computes the delta — which findings are new, which were fixed.
- Returns a [`PrReport`](/sextant-mcp/concepts/report/#prreport-pr-mode)
  whose verdict is computed against new findings only.

This is the mode the GitHub Action uses. It's the slowest of the three
on first run because it grades two trees, but the baseline is cached
per base-SHA — subsequent PRs against the same base reuse it.

## How scope and mode interact

| Rule scope ↓ / Grade mode → | `--diff` | whole-file | `--pr` |
|---|---|---|---|
| `diff` | runs, filtered to hunks | skipped | runs (in head's diff) |
| `file` | runs, finding filtered to diff lines | runs unfiltered | runs unfiltered for both head and base |
| `repo` | skipped | skipped | (roadmap) |

The takeaway: most rules are `file`-scope. Both diff-mode and PR-mode
get filtered findings out of them automatically — there's no need to
write two versions.

## See also

- [`sextant grade`](/sextant-mcp/cli/grade/) — full flag reference.
- [Verdict modes](/sextant-mcp/concepts/verdict/#modes) — absolute vs
  regression.
- [`grade_diff` MCP tool](/sextant-mcp/mcp/tools/grade-diff/).
- [`grade_files` MCP tool](/sextant-mcp/mcp/tools/grade-files/).

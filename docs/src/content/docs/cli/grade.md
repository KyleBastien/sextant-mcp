---
title: sextant grade
description: Grade files in whole-file, diff, or PR mode.
sidebar:
  order: 2
---

`sextant grade` is the workhorse command. It runs every loaded rule
against your code and prints (or writes) a [Report](/sextant-mcp/concepts/report/).

## Usage

```text
sextant grade [PATHS]...

Options:
      --diff
          Switch to diff mode: only findings on changed lines are reported.
      --pr
          PR mode: diff-grade head against a baseline-graded base SHA
          and report only *new* findings introduced by the change.
      --baseline-cache <DIR>
          Directory to read/write per-base-SHA baseline reports.
      --base <BASE>
          Base ref. Default: merge-base with origin/main, falling back to HEAD~1.
      --head <HEAD>
          Head ref. Default: working tree (with index applied).
      --working-tree
          Force diff against the working tree even when --head is set.
      --format <FORMAT>
          Output format: human | json | markdown | sarif | review-json
          [default: human]
      --output <PATH>
          Write the rendered output to PATH instead of stdout.
      --report-json <PATH>
          Side-channel: dump the structured report as JSON to PATH.
      --fail-on <FAIL_ON>
          Severity at which to exit non-zero: never | warn | error
          [default: error]
      --no-llm
          Skip LLM-evaluated rules.
  -h, --help
          Print help.
```

## Modes

`grade` has three mutually exclusive modes:

| Mode | How to invoke | When to use |
|---|---|---|
| **Whole-file** | `sextant grade [PATHS]` (default) | Auditing existing code; one-shot reviews. |
| **Diff** | `sextant grade --diff` | Inner loop and CI on individual edits. Sub-second on typical changes. |
| **PR** | `sextant grade --pr` | Pull-request gating. Reports only findings new in the head. |

`--diff` and `--pr` are incompatible. PR mode implies diff mode under
the hood, plus a baseline grade and a delta computation.

## Inputs

### `[PATHS]...`

Positional file or directory paths to grade. Ignored when `--diff` or
`--pr` is set (the diff itself determines the file set). Defaults to
the current directory.

### `--base <BASE>` and `--head <HEAD>`

Git refs. Used by diff and PR mode.

- **`--base`** defaults to the merge-base with `origin/main`, falling
  back to `HEAD~1` when there's no `origin/main`.
- **`--head`** defaults to the working tree (with the index applied).
  Pass an explicit ref to grade an arbitrary commit.

`--working-tree` forces a working-tree head even when `--head` is set —
useful to diff a stash or feature branch against the working tree.

### `--baseline-cache <DIR>`

PR-mode only. Directory to read/write per-base-SHA baseline reports.
The GitHub Action backs this with `actions/cache` to skip recomputing
the baseline on every PR run. Local users rarely need it.

### `--no-llm`

Drops LLM-evaluated rules at load time, so the grade never touches the
network. Useful for offline use, for CI runs that shouldn't have an
API key, and for the post-edit hook (which uses it by default to keep
latency low).

## Output formats

```sh
sextant grade --format human       # default; coloured terminal
sextant grade --format json        # structured Report
sextant grade --pr --format markdown   # PR-comment markdown
sextant grade --format sarif       # SARIF 2.1.0 for Code Scanning
sextant grade --pr --format review-json  # GitHub PR Reviews API payload
```

| Format | Default for | Notes |
|---|---|---|
| `human` | Interactive use | Coloured, paginated, renders findings inline. |
| `json` | Programmatic consumers | Always emits the full `Report` (or `PrReport`). |
| `markdown` | PR comments | **Only meaningful in `--pr` mode.** Falls back to JSON otherwise. |
| `sarif` | GitHub Code Scanning | Maps each finding to a SARIF result. |
| `review-json` | The GitHub Action | A `Review` payload ready to POST. **PR mode only.** |

`--report-json <PATH>` is independent of `--format`: it always dumps the
structured report (or `PrReport`) to a file. The Action uses this so it
can render a markdown comment while still parsing fields out of JSON.

## Exit codes and `--fail-on`

| `--fail-on` | Exit non-zero on |
|---|---|
| `never` | Nothing — exit code is always 0 unless the CLI itself fails. |
| `warn` | Any warn or error finding. |
| `error` (default) | Any error finding. |

The verdict can also fail: a `request_changes` verdict produces exit
code 1 regardless of `--fail-on` (unless `--fail-on never`).

## Examples

### Inner-loop edit

```sh
sextant grade --diff --working-tree --no-llm
```

Sub-second, offline. The Claude Code plugin's `PostToolUse` hook runs
exactly this.

### CI on a feature branch

```sh
sextant grade --diff --base origin/main --fail-on error
```

### Auditing an unfamiliar repo

```sh
sextant grade --format markdown --output report.md
sextant grade --format sarif --output report.sarif
```

### Dual-output for the GitHub Action

```sh
sextant grade --pr \
  --format markdown --output review.md \
  --report-json report.json \
  --fail-on never
```

`--fail-on never` keeps the CLI's exit code from interfering — the
Action re-evaluates the threshold from the JSON and sets the workflow
status itself.

## See also

- [`sextant rules`](/sextant-mcp/cli/rules/) — list, explain, validate.
- [Scopes](/sextant-mcp/concepts/scopes/) — how grade modes interact
  with rule scopes.
- [Output formats — JSON shape](/sextant-mcp/concepts/report/) — the
  full Report and PrReport schemas.

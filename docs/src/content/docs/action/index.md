---
title: GitHub Action overview
description: Run Sextant on every PR with one workflow file.
sidebar:
  label: Overview
  order: 1
---

The `kylebastien/sextant-mcp/action` action runs Sextant against your
PR: regression-grades the head against the base, posts a single review
comment with the new and fixed findings, and sets the check status from
the verdict.

It uses the same engine as the CLI, so the rules you tune locally are
the rules CI runs.

## Quick start

```yaml
# .github/workflows/sextant.yml
name: Sextant
on:
  pull_request:

permissions:
  contents: read
  pull-requests: write

jobs:
  grade:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          # Required: the action diffs against `base.sha`, which the
          # checkout shallow-clone hides by default.
          fetch-depth: 0
      - uses: kylebastien/sextant-mcp/action@v0.1.0
        with:
          fail-on: error
```

That's it. On the next PR, you'll see a single review comment from the
action with the new and fixed findings, and a check named "Sextant"
that goes red on regression.

## What it does

1. **Resolves the base ref** — input `base`, then `pull_request.base.sha`,
   then `origin/main`.
2. **Restores the baseline cache** — keyed on the base SHA. A miss
   recomputes from `git2` blob reads (no checkout required).
3. **Installs the matching `sextant` binary** from the GitHub Release
   matching `version` (default `latest`), verifying `SHA256SUMS`.
4. **Runs `sextant grade --pr`** — emits a markdown review and a
   `sextant-report.json` for the action to parse.
5. **Posts (or PATCHes) a single PR review comment** carrying a
   `<!-- sextant:review -->` marker so subsequent runs update the same
   thread.
6. **Sets the workflow exit status** from the JSON, not the CLI's exit
   code — so the action can render the review even when the verdict is
   `request_changes`.
7. **Saves the baseline cache** (only on `push` to the default branch).
8. **(optional) Uploads SARIF** to GitHub Code Scanning.

## Detailed reference

- [Inputs](/sextant-mcp/action/inputs/) — every input with defaults.
- [Outputs](/sextant-mcp/action/outputs/) — `verdict`, `new-errors`,
  `new-warnings`.
- [Baseline cache](/sextant-mcp/action/baseline-cache/) — how
  regression grading stays fast.
- [Forks and tokens](/sextant-mcp/action/forks-and-tokens/) —
  permissions, fork PRs, and `pull_request_target`.

## Requirements

- `actions/checkout@v4` with `fetch-depth: 0` so the base SHA is
  reachable.
- `pull-requests: write` permission (default for branches inside the
  same repo; not for forks).
- `contents: read` permission.
- Optionally `security-events: write` for `upload-sarif`.

## With LLM rules

Add a step env block with your provider's API key:

```yaml
- uses: kylebastien/sextant-mcp/action@v0.1.0
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
  with:
    fail-on: error
```

Or pass `no-llm: true` to skip judge-evaluated rules entirely.

## Pinning a version

Pin a specific tag for reproducible builds:

```yaml
- uses: kylebastien/sextant-mcp/action@v0.1.0
  with:
    version: v0.1.0    # match the action version to a CLI release
```

`version` defaults to `latest`, which fetches the latest GitHub Release
of the CLI on every run. For most projects, pinning the action *and*
the CLI version is the right call — Sextant's rules and thresholds are
part of your CI's contract.

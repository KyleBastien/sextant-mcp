---
title: Baseline cache
description: How the action keeps regression grading fast.
sidebar:
  order: 4
---

Regression grading needs two reports: one for the PR head, one for the
base SHA. Re-grading the base on every PR run would double Sextant's
work and stall the inner CI loop. The action sidesteps this by caching
the base report keyed on its SHA.

## Flow

1. **PR opened or pushed.** The action resolves the base SHA from
   `pull_request.base.sha`.
2. **`actions/cache/restore`** with key `sextant-baseline-<base-sha>`.
   Hit → load the JSON `Report`. Miss → fall through to step 3.
3. **Compute the baseline.** `sextant grade --baseline-cache <dir>`
   reads the base tree out of git via `git2` blob reads — no
   `git checkout` required, so the working tree stays at the head
   commit.
4. **Run `sextant grade --pr`** with the cached baseline. The CLI
   diffs the head against the baseline's findings to produce a
   `PrReport`.
5. **`actions/cache/save`** the baseline report — but only on `push`
   to the default branch. PR runs read but never write the cache, so
   the cache stays small and stable.

## Why push-only saves

GitHub Actions caches are scoped per branch with read-through to the
default branch. If PR runs wrote the cache, two PRs against the same
base would race; each would invalidate the other's entry on every
push. Restricting writes to the default branch means:

- Every PR sees a stable baseline, computed once when the base
  commit landed on `main`.
- The cache is sized to one entry per base SHA, not one per
  in-flight PR.

## What you can tune

### `baseline-cache-key`

Default: `sextant-baseline-<base-sha>`. Override only if:

- You want feature branches that diff against the same long-lived
  branch to share a baseline (set them to a shared key).
- You want to forcibly invalidate (bump a counter into the key).

### Disabling the cache

Pass `baseline-cache-key: ''` (empty) to force a fresh baseline grade
on every run. Useful if you suspect cache poisoning or if rule changes
make stored baselines stale faster than the cache can roll over.

## Cache size and eviction

The cache stores serialized JSON `Report`s — typically tens of KB on a
medium-sized repo. GitHub Actions evicts caches not accessed for
seven days (and globally caps a repo at 10 GB), so stale baselines
clean themselves up.

## What if my rules change?

A rule edit on a PR doesn't invalidate the baseline — the action
grades the base SHA with whatever rules were live *at that base*. The
baseline cache holds the resulting report, not the rule state.

When the PR merges and the rule change lands on `main`, the next
`push`-triggered run regrades the base with the new rules and saves a
fresh baseline. From then on, downstream PRs see the updated rule set.

## Local equivalent

The CLI exposes the same machinery via `--baseline-cache <DIR>`:

```sh
sextant grade --pr \
  --base origin/main \
  --baseline-cache .sextant/cache/baseline \
  --report-json sextant-report.json
```

Useful if you want to reproduce the action's behaviour locally — the
JSON output matches what the action posts to the PR.

## See also

- [Inputs → `baseline-cache-key`](/sextant-mcp/action/inputs/#baseline-cache-key).
- [Verdict → regression mode](/sextant-mcp/concepts/verdict/#regression-mode).
- [`sextant grade --pr`](/sextant-mcp/cli/grade/#pr) — local mode.

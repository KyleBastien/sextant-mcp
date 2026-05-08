---
title: Inputs
description: Every input the Sextant Action accepts.
sidebar:
  order: 2
---

| Input | Default | Description |
|---|---|---|
| `version` | `latest` | Sextant release tag to install. Pin a tag (e.g. `v0.1.0`) for reproducibility. |
| `base` | `${{ github.event.pull_request.base.sha }}`, then `origin/main` | Ref to grade against. |
| `head` | working tree | Head ref. Almost always leave default. |
| `fail-on` | `error` | One of `never`, `warn`, `error`. Sets the action's exit status. |
| `format` | `markdown` | `human`, `json`, `markdown`, or `sarif`. Markdown is what `comment` uses. |
| `comment` | `true` | Post (or update) a review comment on the PR. |
| `inline-comments` | `false` | When `true`, also post line-anchored review comments via the GitHub Reviews API. |
| `upload-sarif` | `false` | Run a second pass with `--format sarif` and upload to Code Scanning. Requires `security-events: write`. |
| `no-llm` | `false` | Skip LLM-evaluated rules. Useful when no `[judge]` config / API key is wired in. |
| `baseline-cache-key` | `sextant-baseline-<base>` | Override only if you need to share a baseline across PRs. |
| `github-token` | `${{ github.token }}` | Used for the comment POST. Needs `pull-requests: write`. |

## `version`

`latest` (default) fetches the most recent GitHub Release. Pin to
`v0.1.0` (or whatever tag the rest of your CI expects) for reproducible
builds:

```yaml
- uses: kylebastien/sextant-mcp/action@v0.1.0
  with:
    version: v0.1.0
```

Pinning the action and the CLI version together is the safest config —
Sextant rules and thresholds are part of your CI contract.

## `base` and `head`

Almost always leave defaults. The action picks them up automatically
from the `pull_request` event payload.

Override `base` when:

- You're running on `push` (no `pull_request.base.sha`) — set
  `base: origin/main` or similar.
- You want to grade against a different long-lived branch (e.g. a
  release branch).

## `fail-on`

| Value | Workflow exits non-zero on |
|---|---|
| `never` | Nothing — the action only fails on internal errors. |
| `warn` | Any new warn or error finding. |
| `error` (default) | Any new error finding. |

A `request_changes` verdict will also cause a non-zero exit unless
`fail-on: never`. The check status follows the exit code.

## `format`

What the action puts into the markdown comment. `markdown` is the only
useful value for the comment body — the others fall back to JSON.

Set `format: sarif` if you only care about Code Scanning and don't want
a review comment (combine with `comment: false`).

## `comment` and `inline-comments`

`comment: true` (default) posts a single issue comment carrying a
`<!-- sextant:review -->` marker. Subsequent runs PATCH the same
comment so the PR thread stays clean.

`inline-comments: true` additionally posts line-anchored review
comments via the GitHub Reviews API — one per finding, on the line it
fires on. Useful for code-review workflows where reviewers want to
discuss findings in context. Off by default because chatty PRs are
noisy.

## `upload-sarif`

Runs a second `sextant grade --format sarif` pass and uploads the
result to GitHub Code Scanning. Findings then show up in the security
tab and inline in the diff.

```yaml
- uses: kylebastien/sextant-mcp/action@v0.1.0
  with:
    upload-sarif: true
permissions:
  contents: read
  pull-requests: write
  security-events: write
```

## `no-llm`

Drops LLM-evaluated rules at load time. Use when:

- The repo has no `[judge]` config.
- You want CI to never touch the network.
- You want fast PR runs and accept missing the LLM-rule signal.

## `baseline-cache-key`

The action keys its baseline cache on `sextant-baseline-<base-sha>` by
default. Override when:

- You want to share a baseline across feature branches that diff
  against the same long-lived branch.
- You want to invalidate the cache by changing the key.

## `github-token`

`${{ github.token }}` is fine for branches inside the repo. Fork PRs
get a read-only token — see
[Forks and tokens](/sextant-mcp/action/forks-and-tokens/).

## See also

- [Outputs](/sextant-mcp/action/outputs/) — `verdict`, counts.
- [Baseline cache](/sextant-mcp/action/baseline-cache/) — how the
  cache works.
- [Forks and tokens](/sextant-mcp/action/forks-and-tokens/) —
  permissions and fork PR caveats.

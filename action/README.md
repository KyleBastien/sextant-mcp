# Sextant action

Run [Sextant](https://github.com/kylebastien/sextant-mcp) against your
PR: regression-grade the head against the base, post a single review
comment with the new and fixed findings, set the check status from the
verdict.

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

## Inputs

| Input | Default | Description |
|---|---|---|
| `version` | `latest` | Sextant release tag to install. Pin a tag (e.g. `v0.1.0`) for reproducibility. |
| `base` | `${{ github.event.pull_request.base.sha }}` then `origin/main` | Ref to grade against. |
| `head` | working tree | Head ref. |
| `fail-on` | `error` | One of `never`, `warn`, `error`. Sets the action's exit status. |
| `format` | `markdown` | `human`, `json`, `markdown`, or `sarif`. Markdown is what `comment` uses. |
| `comment` | `true` | Post (or update) a review comment on the PR. |
| `upload-sarif` | `false` | Run a second pass with `--format sarif` and upload to Code Scanning. Needs `security-events: write`. |
| `no-llm` | `false` | Skip LLM-evaluated rules. Useful when no `[judge]` config / API key is wired in. |
| `baseline-cache-key` | `sextant-baseline-<base>` | Override only if you need to share a baseline across PRs. |
| `github-token` | `${{ github.token }}` | Used for the comment POST. Needs `pull-requests: write`. |

## Outputs

| Output | Description |
|---|---|
| `verdict` | `APPROVE` or `REQUEST_CHANGES`. |
| `new-errors` | Count of new error-severity findings introduced by the PR. |
| `new-warnings` | Count of new warn-severity findings introduced by the PR. |

## How it works

1. **Resolve base.** From `inputs.base`, then `pull_request.base.sha`,
   then `origin/main`.
2. **Restore baseline cache.** `actions/cache/restore` keyed by base SHA;
   miss → recompute from `git2` blob reads (no checkout required).
3. **Install sextant.** Download from GitHub Releases, verify
   `SHA256SUMS`, place on `$PATH`.
4. **Run `sextant grade --pr`.** Emits the markdown review and a
   side-channel `sextant-report.json` with the structured `PrReport`.
   The action re-evaluates the threshold from the JSON to decide its
   exit status, so the CLI runs with `--fail-on never`.
5. **Post the review.** `action/scripts/post-review.sh` looks for an
   existing issue comment carrying `<!-- sextant:review -->` and
   PATCHes it; otherwise it POSTs a new comment.
6. **Save baseline cache.** On `push` to the default branch only —
   PR runs read but don't write.
7. **(optional) Upload SARIF** to GitHub Code Scanning so findings show
   up in the security tab too.

## Configuration

The action picks up `.sextant/config.toml` and `.sextant/rules/**/*.md`
from the repo. See the main README for schema. The `[judge]` section is
read here too — set `enabled = true` and an `api_key_env`, and add the
secret to the workflow:

```yaml
- uses: kylebastien/sextant-mcp/action@v0.1.0
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
  with:
    fail-on: error
```

Or pass `no-llm: true` to skip judge-evaluated rules entirely.

## Forks

PRs from forks get a read-only `GITHUB_TOKEN`. The comment step will
log a warning and continue. If you want comments on fork PRs, run the
workflow from `pull_request_target` and review the security
implications carefully — that workflow's job runs with the *base*
branch's secrets.

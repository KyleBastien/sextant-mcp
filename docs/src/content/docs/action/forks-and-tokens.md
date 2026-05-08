---
title: Forks and tokens
description: Permissions, fork PRs, and pull_request_target caveats.
sidebar:
  order: 5
---

The Sextant action posts a review comment, which means it needs write
access to a PR. Inside the same repo that's free; across forks it's a
known GitHub Actions friction point.

## Same-repo PRs

The default workflow works:

```yaml
on:
  pull_request:

permissions:
  contents: read
  pull-requests: write
```

`${{ github.token }}` has `pull-requests: write` on PRs from branches
inside the same repo, so the comment step succeeds.

## Fork PRs

PRs from forks get a **read-only** `GITHUB_TOKEN` regardless of
`permissions:` settings. This is GitHub policy, not a Sextant policy —
fork PRs cannot read repo secrets or write to the base repo.

The action handles this gracefully:

- **The grade still runs.** The action installs `sextant`, reads the
  base SHA, and computes the report.
- **The comment step logs a warning and continues.** No comment is
  posted; the workflow doesn't fail.
- **The check status still updates.** GitHub Actions sets the check
  from the workflow exit code, which doesn't need write permissions.

So fork PRs see the verdict in the check, but no inline review
comment.

## If you want comments on fork PRs

Use `pull_request_target`. The job runs with the **base** branch's
secrets and a writable token, but the working tree is still the fork's
code:

```yaml
on:
  pull_request_target:

permissions:
  contents: read
  pull-requests: write

jobs:
  grade:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.event.pull_request.head.sha }}
          fetch-depth: 0
      - uses: kylebastien/sextant-mcp/action@v0.1.0
        with:
          fail-on: error
```

The two important differences from the same-repo workflow:

1. `pull_request_target` instead of `pull_request`.
2. `actions/checkout` with `ref: ${{ github.event.pull_request.head.sha }}`
   so the action grades the fork's actual code, not the base.

### Security

`pull_request_target` is **dangerous by default**. The job has access
to your repo's secrets and a writable token, but it's running code
written by an outside contributor. If your workflow does anything
beyond grading — running tests, building artifacts, calling external
APIs — that code can be malicious.

Sextant only reads files. It doesn't run the code under review. So
the typical attack vector for `pull_request_target` (e.g. a malicious
`package.json` script running during install) doesn't apply when the
job's only step is `kylebastien/sextant-mcp/action`.

If your job runs anything else — `cargo build`, `npm install`,
`pytest` — review GitHub's
[security guidance for `pull_request_target`](https://securitylab.github.com/research/github-actions-preventing-pwn-requests/)
before using it.

## LLM rules and fork PRs

Even on `pull_request_target`, decide carefully whether to give fork
PRs access to your `ANTHROPIC_API_KEY`. The action passes secrets
straight to the `sextant-mcp` binary, which uses them only to make
LLM-evaluator API calls — but those calls happen against arbitrary code
the contributor wrote. A malicious LLM rule prompt (added in the same
PR) could exfiltrate context to the LLM's hosted endpoint.

Two reasonable middle grounds:

- **Skip LLM rules on fork PRs.** Pass `no-llm: true` on
  `pull_request_target` runs.
- **Run a same-repo `pull_request` workflow with `no-llm: true` for
  forks**, and a separate `pull_request_target` workflow with LLM
  rules only when a maintainer adds a label.

## Custom token

If you'd rather not use `${{ github.token }}` — for instance, to post
comments as a bot account — pass a PAT or GitHub App token:

```yaml
- uses: kylebastien/sextant-mcp/action@v0.1.0
  with:
    github-token: ${{ secrets.SEXTANT_BOT_TOKEN }}
```

The token only needs `pull-requests: write` for the target repo.

## See also

- [Inputs → `github-token`](/sextant-mcp/action/inputs/#github-token).
- [GitHub Actions: Security hardening](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions).

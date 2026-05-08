---
title: Outputs
description: Outputs the Sextant Action exposes to subsequent steps.
sidebar:
  order: 3
---

The action sets three outputs once `sextant grade --pr` finishes. Use
them to chain follow-up steps — gating deploys, notifying Slack, and
similar.

| Output | Description |
|---|---|
| `verdict` | `APPROVE` or `REQUEST_CHANGES`. |
| `new-errors` | Count of new error-severity findings introduced by the PR. |
| `new-warnings` | Count of new warn-severity findings introduced by the PR. |

The action's exit status is the canonical signal — outputs let
downstream steps inspect the result without re-running the grade.

## Example: Slack notification on regression

```yaml
- id: sextant
  uses: kylebastien/sextant-mcp/action@v0.1.0

- if: steps.sextant.outputs.verdict == 'REQUEST_CHANGES'
  uses: slackapi/slack-github-action@v1
  with:
    channel-id: code-review
    slack-message: |
      :warning: PR ${{ github.event.pull_request.number }} introduces
      ${{ steps.sextant.outputs.new-errors }} new error
      and ${{ steps.sextant.outputs.new-warnings }} new warn findings.
      ${{ github.event.pull_request.html_url }}
```

## Example: gating a deploy

```yaml
- id: sextant
  uses: kylebastien/sextant-mcp/action@v0.1.0
  with:
    fail-on: never        # let the deploy step decide

- name: Deploy
  if: steps.sextant.outputs.verdict == 'APPROVE'
  run: ./scripts/deploy.sh
```

`fail-on: never` keeps the action exit code at 0 even on regression,
so the workflow doesn't short-circuit; the deploy step then guards on
the output explicitly.

## Example: counts in a job summary

```yaml
- id: sextant
  uses: kylebastien/sextant-mcp/action@v0.1.0

- run: |
    echo "## Sextant" >> "$GITHUB_STEP_SUMMARY"
    echo "Verdict: ${{ steps.sextant.outputs.verdict }}" >> "$GITHUB_STEP_SUMMARY"
    echo "New errors: ${{ steps.sextant.outputs.new-errors }}" >> "$GITHUB_STEP_SUMMARY"
    echo "New warnings: ${{ steps.sextant.outputs.new-warnings }}" >> "$GITHUB_STEP_SUMMARY"
```

## See also

- [Inputs](/sextant-mcp/action/inputs/) — how to feed the action.
- [Baseline cache](/sextant-mcp/action/baseline-cache/) — what the
  counts are computed against.

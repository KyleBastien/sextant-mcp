#!/usr/bin/env bash
# Post (or update) a Sextant review comment on the current PR.
#
# We use *issue comments* rather than PR reviews because issue comments
# are PATCHable — that lets a re-run update the same comment in place
# instead of stacking N reviews on the PR. The comment we own is
# identified by a `<!-- sextant:review -->` marker that the rendered
# markdown already includes.
#
# Inputs:
#   $1            Path to the rendered markdown file.
#   GH_TOKEN      Token with `pull-requests: write`.
#   REPO          owner/repo.
#   PR_NUMBER     PR number.
#
# Exits 0 on success. Failures here are reported but the action does not
# abort the broader run — losing a comment shouldn't fail a PR check.

set -euo pipefail

body_file="${1:?usage: post-review.sh <markdown-file>}"

if [ ! -s "$body_file" ]; then
  echo "::warning::review body $body_file is empty; skipping" >&2
  exit 0
fi

marker='<!-- sextant:review -->'

# Find a prior comment by us that contains the marker. The default token
# author is `github-actions[bot]`; matching on the marker is more robust
# than matching the user.
existing=$(
  gh api -H "Accept: application/vnd.github+json" \
    "repos/$REPO/issues/$PR_NUMBER/comments?per_page=100" \
    --jq ".[] | select(.body | contains(\"$marker\")) | .id" \
  | head -n1
)

# `gh api` accepts a JSON body via -F body=@file when prefixed correctly.
# We use --input - and feed JSON we build with jq to avoid shell-quoting
# pitfalls with multi-line markdown.
payload=$(jq -Rs '{body: .}' < "$body_file")

if [ -n "$existing" ]; then
  echo "updating existing review comment $existing"
  echo "$payload" | gh api --method PATCH \
    -H "Accept: application/vnd.github+json" \
    "repos/$REPO/issues/comments/$existing" \
    --input - >/dev/null
else
  echo "creating new review comment"
  echo "$payload" | gh api --method POST \
    -H "Accept: application/vnd.github+json" \
    "repos/$REPO/issues/$PR_NUMBER/comments" \
    --input - >/dev/null
fi

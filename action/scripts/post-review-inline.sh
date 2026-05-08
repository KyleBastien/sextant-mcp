#!/usr/bin/env bash
# Post a Sextant review with inline comments anchored on changed lines.
#
# This script consumes the JSON payload sextant emits with
# `--format review-json` (already shaped for GitHub's API), and POSTs it
# to `repos/<owner>/<repo>/pulls/<n>/reviews`.
#
# A failure to attach an inline comment (most commonly: line not in the
# PR diff -> 422) does NOT fail the action — we strip those comments
# and retry once. As a final fallback, we drop all comments and post
# just the body so the verdict still surfaces.
#
# Inputs:
#   $1            Path to the review-JSON file from sextant.
#   GH_TOKEN      Token with `pull-requests: write`.
#   REPO          owner/repo.
#   PR_NUMBER     PR number.

set -euo pipefail

payload="${1:?usage: post-review-inline.sh <review-json-file>}"

if [ ! -s "$payload" ]; then
  echo "::warning::review payload $payload is empty; skipping" >&2
  exit 0
fi

post() {
  local file="$1"
  gh api --method POST \
    -H "Accept: application/vnd.github+json" \
    "repos/$REPO/pulls/$PR_NUMBER/reviews" \
    --input "$file"
}

# Try once with inline comments. A 422 here means at least one of the
# comments points at a line outside the PR diff — drop the comments
# and retry.
if post "$payload" 2>/tmp/sextant-review.err; then
  echo "review posted with inline comments"
  exit 0
fi

if ! grep -q "422" /tmp/sextant-review.err; then
  cat /tmp/sextant-review.err >&2
  echo "::error::review POST failed; see above" >&2
  exit 1
fi

echo "::warning::inline comments were rejected (likely outside diff); retrying with comments stripped" >&2
stripped=$(mktemp)
trap 'rm -f "$stripped"' EXIT
jq '.comments = []' "$payload" > "$stripped"
post "$stripped"
echo "review posted (body-only fallback)"

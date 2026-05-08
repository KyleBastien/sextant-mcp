#!/usr/bin/env bash
# Stop hook: gate turn-end on a clean Sextant grade.
#
# Default behavior is *advisory*: we surface findings to the agent but
# allow stop. Set `SEXTANT_ENFORCE_ON_STOP=1` to flip to *enforcing*
# mode — then a `request_changes` verdict blocks stop and forces the
# agent to keep working.
#
# Hook protocol:
#   * exit 0 + no JSON -> allow stop, no message
#   * stdout JSON with decision=block -> block stop, feed `reason` to
#     the agent as additional context
#   * exit 2 -> hard block (we never use this; PR-level stops should
#     not crash the harness)

set -euo pipefail

if [ "${SEXTANT_DISABLE_STOP:-0}" = "1" ]; then
  exit 0
fi

if ! command -v sextant >/dev/null 2>&1; then
  exit 0
fi

# Drain stdin from the harness even though we don't read it.
cat >/dev/null || true

if [ ! -d .sextant ]; then
  exit 0
fi

report=$(mktemp)
trap 'rm -f "$report"' EXIT

if ! sextant grade --diff --working-tree --no-llm \
    --format json --output "$report" --fail-on never >/dev/null 2>&1; then
  # Sextant itself errored; let the agent stop rather than getting
  # stuck in a loop on a misconfigured repo.
  exit 0
fi

verdict=$(jq -r '.verdict.kind // .verdict' "$report" 2>/dev/null || echo approve)
errors=$(jq -r '.counts.error // 0' "$report")
warns=$(jq -r '.counts.warn // 0' "$report")

if [ "$verdict" = "approve" ]; then
  exit 0
fi

enforce="${SEXTANT_ENFORCE_ON_STOP:-0}"

top=$(jq -r '.findings | .[:5] | .[] |
  "  \(.severity): \(.path):\(.line // 0)  \(.rule_id) — \(.message)"' "$report")

if [ "$enforce" = "1" ] || [ "$enforce" = "true" ]; then
  reason=$(printf 'Sextant verdict: REQUEST_CHANGES (%d errors, %d warnings). Address these before ending the turn:\n%s\n' \
    "$errors" "$warns" "$top")
  jq -Rn --arg r "$reason" '{decision:"block", reason:$r}'
  exit 0
fi

# Advisory mode: don't block, but make the verdict visible in case the
# agent wants to volunteer one more pass.
context=$(printf 'Sextant verdict: REQUEST_CHANGES (%d errors, %d warnings). Advisory only — `SEXTANT_ENFORCE_ON_STOP=1` to enforce.\n%s\n' \
  "$errors" "$warns" "$top")
printf '{"hookSpecificOutput":{"hookEventName":"Stop","additionalContext":%s}}\n' \
  "$(jq -Rn --arg s "$context" '$s')"

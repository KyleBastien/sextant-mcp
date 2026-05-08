#!/usr/bin/env bash
# PostToolUse hook (matcher: Edit|Write|MultiEdit). Run a fast
# `grade_diff` against the working tree and surface any new findings to
# the agent so it self-corrects without waiting to be asked.
#
# Reads the tool-use payload from stdin; we only need it to confirm the
# touched file is under cwd. Exits 0 (non-blocking) on every path —
# this hook informs, it doesn't fail tool calls.

set -euo pipefail

if [ "${SEXTANT_DISABLE_POST_EDIT:-0}" = "1" ]; then
  exit 0
fi

if ! command -v sextant >/dev/null 2>&1; then
  exit 0
fi

# Drain stdin so Claude isn't blocked on the pipe even if we don't use
# the payload.
input=$(cat || true)

# Best-effort guard: skip when no .sextant/ exists. New repos don't
# need to pay the cost on every edit.
if [ ! -d .sextant ]; then
  exit 0
fi

report=$(mktemp)
trap 'rm -f "$report"' EXIT

# `--no-llm` keeps the hook fast and offline; deterministic rules are
# enough for the inner edit loop. Users who want LLM checks can grade
# explicitly via the slash command.
if ! sextant grade --diff --working-tree --no-llm \
    --format json --output "$report" --fail-on never >/dev/null 2>&1; then
  exit 0
fi

# Pull verdict + counts. If everything is clean, stay silent — Claude
# doesn't need a "looks good" notification on every keystroke.
verdict=$(jq -r '.verdict.kind // .verdict' "$report" 2>/dev/null || echo unknown)
errors=$(jq -r '.counts.error // 0' "$report")
warns=$(jq -r '.counts.warn // 0' "$report")
total=$((errors + warns))

if [ "$total" = "0" ] && [ "$verdict" = "approve" ]; then
  exit 0
fi

# Render up to 5 findings so we don't flood context.
top=$(jq -r '.findings | .[:5] | .[] |
  "  \(.severity): \(.path):\(.line // 0)  \(.rule_id) — \(.message)"' "$report")

context=$(printf 'Sextant grade_diff (post-edit): %s — %d errors, %d warnings.\n%s\n' \
  "$verdict" "$errors" "$warns" "$top")

# `additionalContext` is added to Claude's view; the agent can choose
# to act on it. We never block the tool call — that's `Stop`'s job.
printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":%s}}\n' \
  "$(jq -Rn --arg s "$context" '$s')"

# Echo the unused stdin to a discarded variable to silence shellcheck.
: "${input:-}"

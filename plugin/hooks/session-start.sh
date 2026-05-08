#!/usr/bin/env bash
# SessionStart hook: print a one-line summary of currently loaded rules
# so the agent knows what it's being graded against.
#
# Output goes to stdout as JSON with `additionalContext` — Claude reads
# it as part of the system prompt for the new session. Failures here
# are silent; the session should never refuse to start because Sextant
# can't run.

set -euo pipefail

if [ "${SEXTANT_DISABLE_SESSION_START:-0}" = "1" ]; then
  exit 0
fi

if ! command -v sextant >/dev/null 2>&1; then
  exit 0
fi

# `--cwd` doesn't exist; rely on the hook running with the project root
# as the cwd, which Claude Code does by default.
if ! summary=$(sextant rules list 2>/dev/null); then
  exit 0
fi

count=$(printf '%s\n' "$summary" | wc -l | tr -d ' ')
if [ "$count" = "0" ]; then
  exit 0
fi

# Top-3 rule ids for flavor, keeps the message short.
top=$(printf '%s\n' "$summary" \
  | head -n3 \
  | awk -F'\t' 'NR>1{printf ", "} {printf "%s", $1}')

context="Sextant: $count rules loaded (e.g. $top). Use \`grade_diff\` after edits; \`explain_rule <id>\` for details."

# Hook output protocol: print a JSON object on stdout.
printf '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":%s}}\n' \
  "$(jq -Rn --arg s "$context" '$s')"

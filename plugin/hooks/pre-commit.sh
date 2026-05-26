#!/usr/bin/env bash
# Sample git pre-commit hook: gate commits on a clean Sextant grade.
#
# This is NOT a Claude Code plugin hook — it's a *git* hook, run by git
# itself when you `git commit`. Install it as `.git/hooks/pre-commit`
# (chmod +x) or wire it into husky / pre-commit framework. See the
# plugin README for installation options.
#
# Behaviour:
#   * skips silently if `sextant` isn't on PATH or the repo has no
#     `.sextant/` directory
#   * runs `sextant grade --diff --working-tree --no-llm` and exits with
#     its status — non-zero blocks the commit
#   * `--no-llm` keeps it fast and offline; flip to LLM-aware grading by
#     dropping the flag if you want
#
# Bypass with `git commit --no-verify` when you really need to land
# something dirty. Use sparingly.

set -euo pipefail

if [ "${SEXTANT_SKIP_PRECOMMIT:-0}" = "1" ]; then
  exit 0
fi

if ! command -v sextant >/dev/null 2>&1; then
  echo "sextant: not on PATH, skipping pre-commit grade" >&2
  exit 0
fi

if [ ! -d .sextant ]; then
  exit 0
fi

# `--fail-on warn` matches a strict gate (any warn or error fails). Drop
# to `--fail-on error` if you only want errors to block commits, or
# `--fail-on never` to make the hook advisory.
exec sextant grade --diff --working-tree --no-llm --fail-on warn

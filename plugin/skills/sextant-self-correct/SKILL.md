---
name: sextant-self-correct
description: |
  Use when wrapping up a coding task and Sextant has surfaced findings.
  Describes the grade → fix → re-grade loop, how to budget passes, and
  when to stop trying.
---

# Sextant self-correction loop

Sextant is feedback, not just a status check. When findings appear,
work through them before ending the turn.

## The loop

1. Call `grade_diff` (cheap; `--diff` mode against the base).
2. If `verdict == approve`, stop. Done.
3. Otherwise, pick the highest-severity finding (errors first, then
   warns). For ties, take the one closest to the lines you just
   touched — that's almost always the cause.
4. Read the rule body via `explain_rule` if the rule id is unfamiliar.
5. Apply the smallest plausible fix. Don't refactor the universe.
6. Re-run `grade_diff`. Go to (2).

## Budget

Cap at **3 self-correct passes** by default. If the verdict is still
`request_changes` after 3, surface the remaining findings to the user
and stop. Looping forever wastes their time and your context.

A pass that *increases* finding count is a regression — back out the
change and try a different angle, or ask the user.

## When a finding is wrong

Sextant rules can produce false positives, especially LLM-evaluated
ones. If you genuinely disagree:

- Say so out loud, briefly. Cite the rule id and the line.
- Don't silently ignore — the user wants to know you noticed.
- Continue with the next finding. One stuck finding shouldn't sink the
  whole batch.

## Severity meanings, briefly

- `error` — blocks the verdict by default. Fix or explain.
- `warn` — advisory. Fix if cheap, mention if not.
- `info` — informational. Mention only if the user is asking for
  thoroughness.

## Don't

- Don't disable rules or edit `.sextant/config.toml` to silence
  findings unless the user explicitly asks.
- Don't add `// TODO`-style suppressions — Sextant doesn't honor them
  and the next agent will hit the same finding anyway.
- Don't grade with `grade_files` in a tight loop. It's slow.

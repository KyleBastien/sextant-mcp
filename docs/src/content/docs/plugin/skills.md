---
title: Skills
description: The three skills the plugin loads into Claude Code sessions.
sidebar:
  order: 2
---

The plugin ships three skills. Skills are markdown files the plugin
host injects into the agent's context when their descriptions match
the user's request — so the agent learns *when* and *how* to grade
without needing to be told.

| Skill | Triggers when… |
|---|---|
| [`sextant-grade`](#sextant-grade) | The user asks for code-quality feedback, a code review, or a pre-commit check, or the agent needs to interpret findings. |
| [`sextant-self-correct`](#sextant-self-correct) | The agent is wrapping up a coding task and Sextant has surfaced findings. |
| [`sextant-author-rule`](#sextant-author-rule) | The user wants to add a new Sextant rule, codify a preference, or migrate a hand-grep into a permanent check. |

The skill bodies are `plugin/skills/<name>/SKILL.md` in the repo. Read
them directly for the full text.

## `sextant-grade`

Teaches the agent the basics:

- When to call `grade_diff` (inner edit loop, after each meaningful
  change) vs `grade_files` (thorough review, slower).
- How to read a [Report](/sextant-mcp/concepts/report/): start with
  `summary`, then walk findings in severity order.
- Severity meanings: `error` blocks by default, `warn` is advisory,
  `info` is signal-only.
- When **not** to grade: during exploration, on docs-only edits, after
  every keystroke (the hook handles that).

## `sextant-self-correct`

The grade → fix → re-grade loop with a budget:

1. Call `grade_diff`.
2. If `verdict == approve`, stop.
3. Otherwise pick the highest-severity finding (errors first, then
   warns; ties broken by proximity to the edit).
4. `explain_rule` if the rule id is unfamiliar.
5. Apply the smallest plausible fix — don't refactor the universe.
6. Re-grade. Go to (2).

Caps at **three self-correct passes** by default. If still
`request_changes` after three, surface remaining findings to the user
and stop. A pass that *increases* finding count is a regression — back
out the change.

When a finding is a false positive: say so, cite the rule id, move on.
Don't disable rules to silence findings.

## `sextant-author-rule`

The `.sextant/rules/<name>.md` schema — frontmatter fields, evaluator
types (`regex`, `llm`), and the validation flow:

1. Validate: `sextant rules check .sextant/rules/<name>.md`
2. Confirm load: `sextant rules list | grep <id>`
3. Try it: `sextant grade` and look for the new rule.
4. Read it back: `sextant rules explain <id>`.

The rule body becomes user-facing docs (shown by `explain_rule`), so
treat it as documentation, not just code.

## Reading the skill files directly

```sh
ls plugin/skills/
# sextant-author-rule/
# sextant-grade/
# sextant-self-correct/

cat plugin/skills/sextant-grade/SKILL.md
```

Each skill has a YAML frontmatter `description` field — that's what
the plugin host matches against the user's request to decide whether
to inject the skill. Tweak the descriptions in your fork if you want
to broaden or narrow when each fires.

## See also

- [Commands](/sextant-mcp/plugin/commands/) — explicit user-invoked
  commands.
- [Pre-commit hook](/sextant-mcp/plugin/precommit-hook/) — the
  commit-time gate.
- [Authoring rules](/sextant-mcp/rules/authoring/) — full schema for
  the `sextant-author-rule` skill.

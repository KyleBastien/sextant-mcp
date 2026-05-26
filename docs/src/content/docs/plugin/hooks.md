---
title: Hooks
description: SessionStart and PostToolUse hooks that wire Sextant into the edit loop, plus a sample git pre-commit hook for the hard gate.
sidebar:
  order: 4
---

Hooks are why this is a plugin and not just an MCP server. They wire
grading into the agent's lifecycle so feedback arrives without the
agent having to remember to ask for it.

| Hook | Fires when… | Default behaviour |
|---|---|---|
| [SessionStart](#sessionstart) | Claude Code session opens. | Prints a one-line summary of loaded rules. |
| [PostToolUse](#posttooluse) | After Edit / Write / MultiEdit. | Silent `grade_diff` in the background, feeds findings to the agent. |

The Claude hook scripts live at `plugin/hooks/session-start.sh` and
`plugin/hooks/post-edit-grade.sh`. The plugin also ships a sample
**git** pre-commit hook at `plugin/hooks/pre-commit.sh` — see
[Pre-commit hook](/sextant-mcp/plugin/precommit-hook/) for the hard
gate that blocks commits on a dirty grade.

## SessionStart

Prints a one-line summary of the rules currently loaded for the repo
so the agent knows what it's being graded against:

```text
Sextant: 7 builtin + 2 repo rules loaded for kylebastien/sextant-mcp
```

No-op when:

- `sextant` isn't on `PATH`.
- The repo has no `.sextant/` directory.

This hook is purely informational — it doesn't run the grader, just
queries `sextant rules list`.

Disable: `export SEXTANT_DISABLE_SESSION_START=1`.

## PostToolUse

After every `Edit`, `Write`, or `MultiEdit` call, runs
`sextant grade --diff --working-tree --no-llm` in the background. Two
key choices:

- **`--no-llm`** — keeps the hook fast and offline. Explicit grades via
  the slash command can opt back in.
- **`--working-tree`** — grades unstaged + staged changes against the
  merge-base.

If the grade returns findings, they're fed back to the agent as
context — visible to the model on its next turn. If the grade is
clean, the hook stays silent (no spam in the conversation).

Skipped automatically when `.sextant/` doesn't exist — new repos don't
pay the cost on every edit. Also skipped on first commits where
`HEAD~1` doesn't resolve.

Typical latency: well under a second on a small change. The hook runs
in the background, so it doesn't block the edit from completing.

Disable: `export SEXTANT_DISABLE_POST_EDIT=1`.

## Why no Stop hook?

Earlier versions of the plugin shipped a `Stop` hook that re-graded
before the agent ended its turn and could be flipped into a blocking
guardrail with `SEXTANT_ENFORCE_ON_STOP=1`. That hook is gone:

- Blocking turn-end produced dead-end loops when an LLM rule kept
  flagging the same line and the agent couldn't make it happy. The
  fix usually wasn't in the agent's reach.
- It tried to be a commit gate in the wrong place. `git commit` is the
  natural integration point for "this diff is not allowed to land" —
  it's the one the rest of your toolchain already understands.

The replacement is the [git pre-commit
hook](/sextant-mcp/plugin/precommit-hook/) — same `grade_diff` check,
ran at the right moment, with bypass semantics
(`git commit --no-verify`) the team already knows.

## All-or-nothing in the manifest

The plugin manifest lists every Claude hook. To skip individual hooks
permanently, fork the repo and edit `plugin/.claude-plugin/plugin.json`.
The env-var escape hatches are the recommended way for one-off
toggling.

## Disabling all hooks at once

Uninstall the plugin, then add the MCP server by hand —
[Use with Claude Code → Manual MCP config](/sextant-mcp/mcp/claude-code/#manual-mcp-config).
You keep the tools, lose the automation.

## See also

- [Pre-commit hook](/sextant-mcp/plugin/precommit-hook/) — the hard
  gate at commit time.
- [Skills](/sextant-mcp/plugin/skills/) — the agent-side companion to
  hooks.
- [`sextant grade`](/sextant-mcp/cli/grade/) — what hooks run under
  the hood.

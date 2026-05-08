---
title: Hooks
description: SessionStart, PostToolUse, and Stop hooks that wire Sextant into the edit loop.
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
| [Stop](#stop) | Before the agent ends its turn. | Final `grade_diff`. Advisory by default; enforcing optionally. |

The hook scripts live at `plugin/hooks/*.sh`.

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

## Stop

Before the agent ends its turn, runs the same `grade_diff` once more.

The behaviour depends on `SEXTANT_ENFORCE_ON_STOP`:

### Advisory (default)

Surfaces the verdict and findings as context. Doesn't block the stop —
the agent can volunteer one more pass if it wants.

```text
[Sextant] verdict: request_changes
  warn  src/handlers.rs:88  builtin.size.fn-length
        Function `dispatch` is 78 lines (warn at 60)
```

The agent reads this on its next turn (which doesn't happen
automatically, since the user is now in control). If the user prompts
again, the agent has the findings in context.

### Enforcing

```sh
export SEXTANT_ENFORCE_ON_STOP=1
```

A `request_changes` verdict now **blocks** the stop — the hook returns
a non-zero exit code, the plugin host treats it as a stop reason, and
the agent gets the findings back as the reason it can't end the turn.
The agent continues iterating until either:

- The verdict flips to `approve`.
- The agent gives up (typical models retry up to a few times before
  surfacing the failure to the user).

This converts Sextant from a code-review companion into a guardrail.
It also costs more tokens — the agent burns turns fixing findings.

See [Enforcing mode](/sextant-mcp/plugin/enforcing-mode/) for when to
turn this on.

Disable: `export SEXTANT_DISABLE_STOP=1` (overrides
`SEXTANT_ENFORCE_ON_STOP`).

## All-or-nothing in the manifest

The plugin manifest lists every hook. To skip individual hooks
permanently, fork the repo and edit `plugin/manifest.json`. The env-var
escape hatches are the recommended way for one-off toggling.

## Disabling all hooks at once

Uninstall the plugin, then add the MCP server by hand —
[Use with Claude Code → Manual MCP config](/sextant-mcp/mcp/claude-code/#manual-mcp-config).
You keep the tools, lose the automation.

## See also

- [Enforcing mode](/sextant-mcp/plugin/enforcing-mode/) — turn the
  stop hook into a blocker.
- [Skills](/sextant-mcp/plugin/skills/) — the agent-side companion to
  hooks.
- [`sextant grade`](/sextant-mcp/cli/grade/) — what hooks run under
  the hood.

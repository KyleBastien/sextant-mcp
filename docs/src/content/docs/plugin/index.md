---
title: Claude Code plugin overview
description: Bundle Sextant into a Claude Code session — MCP server, skills, slash commands, and hooks.
sidebar:
  label: Overview
  order: 1
---

The Sextant Claude Code plugin bundles the MCP server, three skills the
agent auto-loads, three slash commands, and three hooks that turn
grading into a live signal during the edit loop.

## Install

You need the `sextant` and `sextant-mcp` binaries on `PATH` first. See
[Installation](/sextant-mcp/getting-started/installation/).

Then, from a Claude Code session:

```text
/plugin marketplace add kylebastien/sextant-mcp
/plugin install sextant@kylebastien/sextant-mcp
```

The plugin lives at `plugin/` in the
[sextant-mcp repo](https://github.com/kylebastien/sextant-mcp), so the
marketplace is the repo itself. After install, **restart the session**
to pick up the hooks.

## What's in the box

| Piece | What it does |
|---|---|
| [MCP server](/sextant-mcp/mcp/) | Registers `sextant-mcp` so the agent has `grade_diff`, `grade_files`, `list_rules`, `explain_rule`, `get_config`. |
| [Skills](/sextant-mcp/plugin/skills/) | Three auto-loaded skills the agent uses to know *when* and *how* to grade. |
| [Slash commands](/sextant-mcp/plugin/commands/) | Three `/sextant-*` commands you can invoke explicitly. |
| [Hooks](/sextant-mcp/plugin/hooks/) | `SessionStart`, `PostToolUse`, `Stop` — pull grading into the edit loop without explicit prompts. |

## Why a plugin and not just an MCP server?

The MCP server alone gives the agent the *ability* to grade. The
plugin adds the *behaviour*: skills tell the agent when to call which
tool; the post-edit hook grades silently after every change so the
agent gets feedback without burning tokens on tool calls; the stop
hook can act as a guardrail that blocks the turn until the verdict
flips.

If you just want the tools, [add the MCP server by hand](/sextant-mcp/mcp/claude-code/#manual-mcp-config).
If you want the full self-correcting edit loop, install the plugin.

## Disabling pieces

Plugin hooks are all-or-nothing in the manifest, but each script
respects an env-var escape hatch:

```sh
# Disable post-edit grading without uninstalling the plugin.
export SEXTANT_DISABLE_POST_EDIT=1

# Same pattern for the others:
export SEXTANT_DISABLE_STOP=1
export SEXTANT_DISABLE_SESSION_START=1
```

Set in your shell or in Claude Code's env config.

To opt out of a single piece more permanently — fork the repo and edit
`plugin/manifest.json`. Skills are loaded by name; commands by file.

## Authoring

- **Skills** live at `plugin/skills/<name>/SKILL.md`.
- **Commands** live at `plugin/commands/<name>.md`.
- **Hooks** live at `plugin/hooks/*.sh`.

Each is plain markdown or bash — no compile step. After editing,
reload the plugin (`/plugin reload sextant`) or restart the session.

## Troubleshooting

**"sextant: command not found" in hook output.** The plugin host
inherits your shell `PATH`. Either install Sextant somewhere already
on `PATH` (`~/.local/bin`, `/usr/local/bin`) or add the install
directory explicitly to your shell rc.

**Hooks fire but produce no output.** That's the silent-on-clean
behaviour — the post-edit and stop hooks only speak when there are
findings. Run `sextant grade --diff --working-tree` manually to
confirm.

**`HEAD~1` errors on a fresh repo.** The diff hook needs a base
commit. The first commit of a repo has no `HEAD~1`; Sextant returns a
friendly "no default base" error and the hook exits silently.

**Enforce mode blocked stop and the agent is stuck.** Either fix the
findings or unset `SEXTANT_ENFORCE_ON_STOP` for one turn:

```sh
SEXTANT_ENFORCE_ON_STOP=0 claude
```

See [Enforcing mode](/sextant-mcp/plugin/enforcing-mode/) for the
trade-offs.

## See also

- [Skills](/sextant-mcp/plugin/skills/) — the three auto-loaded
  skills.
- [Commands](/sextant-mcp/plugin/commands/) — `/sextant-grade`,
  `/sextant-init`, `/sextant-explain`.
- [Hooks](/sextant-mcp/plugin/hooks/) — `SessionStart`, `PostToolUse`,
  `Stop`.
- [Enforcing mode](/sextant-mcp/plugin/enforcing-mode/) — turn the
  stop hook into a guardrail.

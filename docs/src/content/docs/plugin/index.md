---
title: Claude Code plugin overview
description: Bundle Sextant into a Claude Code session — MCP server, skills, slash commands, and a git pre-commit hook.
sidebar:
  label: Overview
  order: 1
---

The Sextant Claude Code plugin bundles the MCP server, three skills the
agent auto-loads, three slash commands, and a sample git pre-commit
hook that blocks commits on a dirty grade.

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
marketplace is the repo itself.

## What's in the box

| Piece | What it does |
|---|---|
| [MCP server](/sextant-mcp/mcp/) | Registers `sextant-mcp` so the agent has `grade_diff`, `grade_files`, `list_rules`, `explain_rule`, `get_config`. |
| [Skills](/sextant-mcp/plugin/skills/) | Three auto-loaded skills the agent uses to know *when* and *how* to grade. |
| [Slash commands](/sextant-mcp/plugin/commands/) | Three `/sextant-*` commands you can invoke explicitly. |
| [Pre-commit hook](/sextant-mcp/plugin/precommit-hook/) | Sample git pre-commit script that blocks commits on a dirty grade. |

## Why a plugin and not just an MCP server?

The MCP server alone gives the agent the *ability* to grade. The
plugin adds the *behaviour*: skills tell the agent when to call which
tool, slash commands let you invoke grading explicitly, and the
sample [git pre-commit hook](/sextant-mcp/plugin/precommit-hook/) is
the gate that blocks commits until the diff is clean.

If you just want the tools, [add the MCP server by hand](/sextant-mcp/mcp/claude-code/#manual-mcp-config).
If you want the skills, commands, and pre-commit gate, install the
plugin.

## Why no Claude hooks?

Earlier versions of the plugin shipped `SessionStart`, `PostToolUse`,
and `Stop` hooks. They're gone:

- The `Stop` hook (with `SEXTANT_ENFORCE_ON_STOP=1`) produced
  dead-end loops when an LLM rule kept flagging the same line and
  the agent couldn't make it happy.
- The `PostToolUse` hook surfaced findings the agent could just talk
  past, and burned tokens every keystroke.
- `git commit` is the natural integration point for "this diff is
  not allowed to land" — the gate runs once per commit and aborts
  the commit outright when it fires.

The replacement is the [pre-commit
hook](/sextant-mcp/plugin/precommit-hook/) — same `grade_diff`
check, ran at the right moment, strict by design with no env-var
escape hatch.

## Authoring

- **Skills** live at `plugin/skills/<name>/SKILL.md`.
- **Commands** live at `plugin/commands/<name>.md`.
- **Pre-commit script** lives at `plugin/hooks/pre-commit.sh`.

Each is plain markdown or bash — no compile step. After editing
skills or commands, reload the plugin (`/plugin reload sextant`) or
restart the session.

## Troubleshooting

**"sextant: command not found" when committing.** Your shell `PATH`
doesn't include the install dir. Install Sextant somewhere already on
`PATH` (`~/.local/bin`, `/usr/local/bin`) or add the install
directory explicitly to your shell rc so commits launched from `git`
inherit it.

**`HEAD~1` errors on a fresh repo.** The diff grade needs a base
commit. The first commit of a repo has no `HEAD~1`; Sextant returns a
friendly "no default base" error and the hook exits silently.

**Pre-commit hook blocked the commit.** Fix the findings. The gate is
strict by design — there is no env-var escape hatch. If a rule fires
repeatedly on something that isn't a real problem, calibrate the
rule rather than trying to skip the grade.

## See also

- [Skills](/sextant-mcp/plugin/skills/) — the three auto-loaded
  skills.
- [Commands](/sextant-mcp/plugin/commands/) — `/sextant-grade`,
  `/sextant-init`, `/sextant-explain`.
- [Pre-commit hook](/sextant-mcp/plugin/precommit-hook/) — the gate
  at commit time.

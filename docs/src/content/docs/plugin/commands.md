---
title: Slash commands
description: User-invoked /sextant-* commands.
sidebar:
  order: 3
---

The plugin registers three slash commands. Use them when you want to
trigger Sextant explicitly rather than wait for a hook or a skill to
decide.

| Command | What it does |
|---|---|
| [`/sextant-grade [paths]`](#sextant-grade) | Grade the working tree (or specified paths) and summarize. |
| [`/sextant-init`](#sextant-init) | Run `sextant init` in the current repo. |
| [`/sextant-explain <rule-id>`](#sextant-explain) | Print the markdown body for a rule. |

## `/sextant-grade`

```text
/sextant-grade
/sextant-grade src/parser.rs src/lexer.rs
```

With no arguments, the command calls `grade_diff` (working tree vs
merge-base) — the cheap inner-loop grade. With paths, it calls
`grade_files` against just those paths.

The agent then summarizes:

1. **Verdict** — `approve` or `request_changes` and the counts.
2. **Top three findings** — severity, rule id, file:line, message.
3. **Unfamiliar rules** — looks them up via `explain_rule` and
   includes a one-liner.
4. **Next step** — most actionable fix. Doesn't apply fixes unless
   you ask.

## `/sextant-init`

```text
/sextant-init
```

Runs `sextant init` in the current working directory. Idempotent —
files already present are skipped unless you pass `--force` to the CLI
directly (the slash command doesn't take a flag for this, by design).

After it runs, the agent reports what was created and points you at
follow-up commands: `sextant grade`, `sextant rules list`,
`sextant rules explain`.

## `/sextant-explain`

```text
/sextant-explain builtin.size.fn-length
/sextant-explain project.no-unwrap
```

Looks up the rule whose id matches the argument and renders its
markdown body. The body is the authoritative documentation for the
rule — *why* it exists, what trips it, and how to fix a finding.

If the id doesn't exist, the agent falls back to `list_rules` and
offers fuzzy matches. Useful when you've half-remembered an id.

## Authoring your own

Commands live at `plugin/commands/<name>.md`. Each is a markdown file
with YAML frontmatter:

```yaml
---
description: One-line description shown in /help.
argument-hint: "[paths...]"
allowed-tools: ["mcp__sextant__grade_diff", "Bash"]
---
```

The body of the markdown file is the prompt the agent sees when the
command is invoked, with `$ARGUMENTS` substituted.

After editing, reload the plugin (`/plugin reload sextant`) or restart
the session.

## See also

- [Skills](/sextant-mcp/plugin/skills/) — auto-loaded behaviour.
- [Pre-commit hook](/sextant-mcp/plugin/precommit-hook/) — the
  commit-time gate.
- [`sextant grade`](/sextant-mcp/cli/grade/) — the underlying CLI.

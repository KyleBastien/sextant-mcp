---
title: Quickstart
description: Grade a repo with Sextant in five minutes.
sidebar:
  order: 2
---

This walk-through takes a fresh repo from zero to a green Sextant verdict
in under five minutes. It assumes you have `sextant` and `sextant-mcp` on
`PATH` (see [Installation](/sextant-mcp/getting-started/installation/)).

## 1. Bootstrap a config

```sh
cd path/to/your-repo
sextant init
```

This writes a starter `.sextant/` directory:

```
.sextant/
├── config.toml         # verdict thresholds and rule limits
└── rules/              # repo-local rule markdown files (empty by default)
```

The default config is conservative — `max_errors = 0`, file-length warn at
400 lines, error at 800. Tune these later if needed.

## 2. Grade the working tree

```sh
sextant grade
```

Output looks like:

```
Sextant — 12 findings (0 errors, 7 warns, 5 info)

  warn  src/parser.rs:412
        builtin.size.file-length
        File length 412 exceeds warn threshold (400)

  warn  src/handlers.rs:88
        builtin.size.fn-length
        Function `dispatch` is 78 lines (warn at 60)

  …

verdict: approve
```

The default mode grades **whole files**: every line is checked, every
finding listed. Useful for an audit of a repo's debt.

## 3. Grade only what changed

For day-to-day work and CI, you almost always want diff mode:

```sh
sextant grade --diff --base origin/main
```

This restricts findings to lines in your branch's diff. It's fast (sub-
second on most changes) and matches how the GitHub Action and the agent
hooks grade.

## 4. List the rules

```sh
sextant rules list
```

Shows every rule loaded for this repo — built-ins plus anything under
`.sextant/rules/`. To read a rule's full body and fix advice:

```sh
sextant rules explain builtin.size.fn-length
```

## 5. Pick a surface

You're now ready to wire Sextant into a feedback loop:

- **Claude Code:**
  [install the plugin](/sextant-mcp/plugin/) so the agent grades after
  every edit.
- **Claude Desktop / other MCP client:**
  [add the MCP server](/sextant-mcp/mcp/claude-desktop/).
- **CI:**
  [add the GitHub Action](/sextant-mcp/action/) so every PR gets a
  review comment.

## What you just built

- A `.sextant/` config that the CLI, MCP server, and Action all read from.
- Seven built-in rules covering size, complexity, duplication, and tests.
- A grading entry point you can call from a shell, an editor, or CI.

Next, learn the [data model](/sextant-mcp/concepts/) so the report fields
make sense, or jump straight to a [surface](/sextant-mcp/cli/).

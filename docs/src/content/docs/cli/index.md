---
title: CLI overview
description: The sextant binary — top-level commands and global behaviour.
sidebar:
  label: Overview
  order: 1
---

The `sextant` CLI is the same binary used by the GitHub Action and the
Claude Code plugin. Everything Sextant does is reachable from it.

```sh
sextant --help
```

```text
Code grader for AI agent workflows

Usage: sextant <COMMAND>

Commands:
  grade  Grade files. Defaults to whole-file mode; pass `--diff`
         to grade only changed lines, or `--pr` for PR-mode
         regression grading with a baseline cache.
  rules  Rule introspection commands.
  init   Write a `.sextant/` directory with a config and sample rules.
  help   Print this message or the help of the given subcommand(s).
```

## Commands

| Command | What it does |
|---|---|
| [`sextant grade`](/sextant-mcp/cli/grade/) | Grade files. Default whole-file; `--diff` for changed-only; `--pr` for regression. |
| [`sextant rules`](/sextant-mcp/cli/rules/) | List, explain, and validate rules. |
| [`sextant init`](/sextant-mcp/cli/init/) | Bootstrap a `.sextant/` directory. |

## Global behaviour

### Working directory

`sextant` always reads `.sextant/config.toml` and `.sextant/rules/**/*.md`
from the current working directory's repository root. Run it from
anywhere inside the repo.

### Logging

All log output goes to **stderr**. Stdout is reserved for command output
(grade reports, rule lists, init messages) so you can pipe it cleanly.

Set `RUST_LOG` for verbose output:

```sh
RUST_LOG=debug sextant grade
RUST_LOG=sextant_engine=trace sextant grade --diff
```

Defaults to `warn`.

### Exit codes

| Code | When |
|---|---|
| `0` | Verdict was `approve` and no findings exceeded `--fail-on`. |
| `1` | Verdict was `request_changes`, or findings exceeded `--fail-on`. |
| `2` | The CLI itself errored (bad args, IO failure, malformed rule, etc.). |

`grade` is the only command that produces a non-zero exit code on
findings; `rules` and `init` use `0` for success and `2` for errors.

### Config

The config file is `.sextant/config.toml`. The schema is documented
under [Configuration](/sextant-mcp/configuration/).

### Cache

LLM-rule responses live under `.sextant/cache/llm/` keyed by BLAKE3 hash
of the file contents and rule body. Baseline reports for PR mode live
under `.sextant/cache/baseline/`. The whole `.sextant/cache/` tree is
git-ignored by `sextant init`.

## See also

- [`sextant grade`](/sextant-mcp/cli/grade/) — the grading command.
- [`sextant rules`](/sextant-mcp/cli/rules/) — rule introspection.
- [`sextant init`](/sextant-mcp/cli/init/) — bootstrap config.
- [Configuration](/sextant-mcp/configuration/) — `.sextant/config.toml`
  schema.

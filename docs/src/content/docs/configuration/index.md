---
title: Configuration
description: The .sextant/config.toml schema.
sidebar:
  label: Overview
  order: 1
---

Sextant reads `.sextant/config.toml` from the repository root. The
file is optional — every section has sensible defaults — but most
real projects ship one to tune thresholds.

## Bootstrap

```sh
sextant init
```

writes a starter file. Edit it, commit it, and the CLI, the MCP
server, and the GitHub Action all read from the same source.

## Full example

```toml
# Verdict thresholds — what counts as "block this".
[verdict]
max_errors = 0
max_warns = 50

# Size-rule limits.
[size]
file_length_warn = 400
file_length_error = 800
fn_length_warn = 60
fn_length_error = 120
param_count_warn = 6
param_count_error = 10

# Complexity-rule limits.
[complexity]
cyclomatic_warn = 10
cyclomatic_error = 20
nesting_warn = 4
nesting_error = 6

# Token-duplication rule.
[duplication]
min_tokens = 100

# LLM-evaluator config.
[judge]
enabled = true
provider = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
max_concurrency = 4
cache_dir = ".sextant/cache/llm"
```

## Sections

| Section | What it controls |
|---|---|
| [`[verdict]`](/sextant-mcp/configuration/verdict/) | When findings flip the verdict to `request_changes`. |
| [`[size]`](/sextant-mcp/configuration/size/) | Thresholds for size-category built-ins. |
| [`[complexity]`](/sextant-mcp/configuration/complexity/) | Thresholds for complexity-category built-ins. |
| [`[duplication]`](/sextant-mcp/configuration/duplication/) | Token-duplication detection. |
| [`[judge]`](/sextant-mcp/configuration/judge/) | LLM provider config for `llm`-evaluated rules. |

## Skipped paths

Sextant never grades generated, vendored, or prose files. The skip
list is hardcoded into the engine — there is no `[paths]` section to
edit:

- `**/Cargo.lock`, `**/package-lock.json`, `**/yarn.lock`,
  `**/pnpm-lock.yaml`, `**/poetry.lock`, `**/uv.lock`
- `**/target/**`, `**/node_modules/**`, `**/dist/**`, `**/build/**`
- `**/.git/**`, `**/.sextant/cache/**`
- `**/*.md`, `**/*.markdown`, `**/*.mdx` — Markdown is prose; the
  built-in size/complexity rules are tuned for code.

Hiding source from a rule by excluding paths is intentionally not
supported. If a rule fires on code that legitimately shouldn't trigger
it, fix the rule (refine its pattern, add a `not_under` ancestor check
on AST rules) rather than carving out a hole.

## Inspecting the resolved config

```sh
sextant grade --format json | jq '.config'   # not in the report
# Use the MCP `get_config` tool, or:
RUST_LOG=sextant_config=debug sextant grade
```

The MCP [`get_config`](/sextant-mcp/mcp/tools/get-config/) tool
returns the merged config — defaults overlaid by `config.toml` —
which is the easiest way to see what's actually in effect.

## See also

- [`sextant init`](/sextant-mcp/cli/init/) — scaffold a config.
- [`get_config` MCP tool](/sextant-mcp/mcp/tools/get-config/) —
  inspect at runtime.
- [Rules catalog](/sextant-mcp/rules/) — what each section gates.

---
title: MCP server overview
description: The sextant-mcp server — transports, environment, tools.
sidebar:
  label: Overview
  order: 1
---

`sextant-mcp` is the [Model Context Protocol](https://modelcontextprotocol.io/)
server that exposes Sextant's grading engine to agents. It speaks
JSON-RPC 2.0 over either stdio or HTTP and surfaces five tools.

## Tools

| Tool | Purpose |
|---|---|
| [`grade_diff`](/sextant-mcp/mcp/tools/grade-diff/) | Grade just the changed lines. Inner-loop fast. |
| [`grade_files`](/sextant-mcp/mcp/tools/grade-files/) | Grade entire files. Slower, thorough. |
| [`list_rules`](/sextant-mcp/mcp/tools/list-rules/) | List every loaded rule. |
| [`explain_rule`](/sextant-mcp/mcp/tools/explain-rule/) | Fetch a rule's full markdown documentation. |
| [`get_config`](/sextant-mcp/mcp/tools/get-config/) | Return the resolved Sextant config. |

The schemas come straight from the engine's
[Report](/sextant-mcp/concepts/report/) /
[Verdict](/sextant-mcp/concepts/verdict/) types — same data the CLI
returns in `--format json`.

## Transports

### stdio (default)

```sh
sextant-mcp
```

The standard MCP transport: JSON-RPC over stdin/stdout, one message per
line. Logging goes to stderr. This is what Claude Code, Claude Desktop,
and most other MCP clients use.

### HTTP

```sh
sextant-mcp --http 127.0.0.1:7878
```

Newline-delimited JSON-RPC over HTTP at the bound address. Each request
is a single POST to `/`. Useful for clients that don't speak stdio,
debugging via `curl`, or running the server as a long-lived daemon.

```sh
curl -sS http://127.0.0.1:7878/ \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

## Environment

| Variable | Purpose |
|---|---|
| `RUST_LOG` | tracing env-filter. Defaults to `warn`. Try `RUST_LOG=sextant_mcp=debug` while debugging. |
| `ANTHROPIC_API_KEY` | Used by LLM-evaluated rules when `[judge]` selects an Anthropic model. |
| `OPENAI_API_KEY` | Same, for OpenAI models. |
| `SEXTANT_NO_LLM` | If set to `1`, disables LLM evaluators globally — same as the CLI's `--no-llm`. |

The server reads `.sextant/config.toml` and `.sextant/rules/**/*.md`
from the current working directory's repository root, just like the
CLI. Run it from the repo root.

## Setup guides

- [Use with Claude Code](/sextant-mcp/mcp/claude-code/) — easy path is
  the bundled plugin.
- [Use with Claude Desktop](/sextant-mcp/mcp/claude-desktop/) — manual
  config.

## Logging

All log output goes to **stderr**. The stdio transport reserves stdout
for the JSON-RPC protocol, so any println! to stdout would corrupt the
stream. In HTTP mode, stderr still gets the logs.

For verbose tracing during development:

```sh
RUST_LOG=sextant_mcp=debug,sextant_engine=debug sextant-mcp
```

## See also

- [grade_diff](/sextant-mcp/mcp/tools/grade-diff/),
  [grade_files](/sextant-mcp/mcp/tools/grade-files/),
  [list_rules](/sextant-mcp/mcp/tools/list-rules/),
  [explain_rule](/sextant-mcp/mcp/tools/explain-rule/),
  [get_config](/sextant-mcp/mcp/tools/get-config/) — tool reference.
- [Claude Code plugin](/sextant-mcp/plugin/) — bundles this server with
  hooks and skills.
- [Configuration](/sextant-mcp/configuration/) — what the server reads.

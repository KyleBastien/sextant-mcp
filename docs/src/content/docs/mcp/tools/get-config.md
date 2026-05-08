---
title: get_config
description: Return the resolved Sextant configuration as JSON.
sidebar:
  order: 5
---

Return the resolved Sextant configuration — verdict thresholds,
size-rule limits, complexity limits, path excludes, judge settings —
as JSON.

Use this to debug why a rule is firing (or being skipped), or to
confirm a config edit was picked up.

## Input schema

```json
{ "type": "object", "properties": {} }
```

No arguments.

## Output

A JSON object reflecting the merged config: defaults overlaid by
`.sextant/config.toml`. Wrapped in the MCP `content` envelope.

```json
{
	"verdict": {
		"max_errors": 0,
		"max_warns": 50
	},
	"size": {
		"file_length_warn": 400,
		"file_length_error": 800,
		"fn_length_warn": 60,
		"fn_length_error": 120,
		"param_count_warn": 6,
		"param_count_error": 10
	},
	"complexity": {
		"cyclomatic_warn": 10,
		"cyclomatic_error": 20,
		"nesting_warn": 4,
		"nesting_error": 6
	},
	"duplication": {
		"min_tokens": 100
	},
	"paths": {
		"exclude": ["*.pb.go", "vendor/"]
	},
	"judge": {
		"enabled": true,
		"provider": "anthropic",
		"model": "claude-sonnet-4-6",
		"api_key_env": "ANTHROPIC_API_KEY",
		"max_concurrency": 4,
		"cache_dir": ".sextant/cache/llm"
	}
}
```

Fields not present in `config.toml` show their defaults — what you see
is what the engine will actually use.

## When to call

- "Why didn't this finding fire?" — check `paths.exclude` and
  thresholds.
- After editing `config.toml` — confirm the change is loaded.
- When walking an unfamiliar repo — see what thresholds it ships.

## Examples

```json
{ "name": "get_config", "arguments": {} }
```

## Errors

| MCP error code | Cause |
|---|---|
| `-32603` (internal error) | `config.toml` is malformed TOML or violates the schema. |

## See also

- [Configuration](/sextant-mcp/configuration/) — full schema.
- [`list_rules`](/sextant-mcp/mcp/tools/list-rules/) — what's loaded.

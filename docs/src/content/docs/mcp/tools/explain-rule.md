---
title: explain_rule
description: Return the full markdown documentation for a single rule.
sidebar:
  order: 4
---

Return the full markdown documentation for a single rule, including
thresholds and how to fix findings. Always call this after seeing an
unfamiliar `rule_id` in a finding.

Returns raw markdown text, not JSON.

## Input schema

```json
{
	"type": "object",
	"required": ["id"],
	"properties": {
		"id": {
			"type": "string",
			"description": "Rule id (e.g. `builtin.size.fn-length`)."
		}
	}
}
```

## Output

A markdown string wrapped in the MCP `content` envelope:

```json
{
	"content": [
		{
			"type": "text",
			"text": "# Function length (builtin.size.fn-length)\n\nFunctions whose body spans more than the configured number of lines.\n\n# Function length\n\nLong functions tend to bundle multiple responsibilities…"
		}
	]
}
```

The text is the rule's body — the same content shown by
[`sextant rules explain`](/sextant-mcp/cli/rules/#sextant-rules-explain-id).

If the id doesn't exist, the tool returns an MCP error result instead:

```json
{
	"isError": true,
	"content": [
		{ "type": "text", "text": "no rule with id `made-up-id`" }
	]
}
```

## When to call

- Right after seeing a `rule_id` in a finding that the agent doesn't
  recognize. The body almost always contains the fix pattern.
- When deciding whether a finding is a false positive — the body
  documents the rule's intent.
- When authoring a new rule, to read an existing one as a template.

## Examples

```json
{
	"name": "explain_rule",
	"arguments": { "id": "builtin.size.fn-length" }
}
```

```json
{
	"name": "explain_rule",
	"arguments": { "id": "project.no-unwrap" }
}
```

## Errors

| Failure mode | Result |
|---|---|
| Unknown id | `isError: true` with a "no rule with id" message. Not an MCP-level error. |
| Malformed args | MCP error code `-32602` (invalid params). |
| IO failure | MCP error code `-32603` (internal error). |

The `isError` distinction lets agents handle missing-rule cases
without retrying.

## See also

- [`list_rules`](/sextant-mcp/mcp/tools/list-rules/) — find an id.
- [`sextant rules explain`](/sextant-mcp/cli/rules/) — same content,
  CLI version.
- [Rules catalog](/sextant-mcp/rules/) — browse built-ins online.

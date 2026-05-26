---
title: list_rules
description: List every rule loaded for this repository.
sidebar:
  order: 3
---

List every rule loaded for the current repository. Each entry includes
id, name, severity, category, scope, source (`builtin` / `repo` /
`override`), and a one-line description.

Use the id to fetch full markdown documentation via
[`explain_rule`](/sextant-mcp/mcp/tools/explain-rule/).

## Input schema

```json
{ "type": "object", "properties": {} }
```

No arguments.

## Output

A JSON array of rule summaries, wrapped in the MCP `content` envelope:

```json
[
	{
		"id": "builtin.size.fn-length",
		"name": "Function length",
		"description": "Functions whose body spans more than the configured number of lines.",
		"severity": "warn",
		"category": "size",
		"scope": "file",
		"source": "builtin",
		"languages": ["rust", "python", "go", "java", "typescript", "tsx", "javascript"],
		"enabled": true,
		"tags": ["size", "complexity"]
	},
	{
		"id": "project.no-unwrap",
		"name": "No unwrap() in production code",
		"description": "Disallow .unwrap() outside test files.",
		"severity": "warn",
		"category": { "custom": "panics" },
		"scope": "file",
		"source": "repo",
		"languages": ["rust"],
		"enabled": true,
		"tags": ["rust", "panics"]
	}
]
```

The fields mirror rule frontmatter. The `body` field is intentionally
omitted — it's often long, and most callers just want metadata. Use
`explain_rule` to fetch a body.

## When to call

- The agent is unsure which rules apply and wants to filter by category
  or language.
- After authoring a new rule, to confirm it loaded.
- As context for an `explain_rule` follow-up — the agent picks the id
  here, then asks for details.

Useful as a first call in a session — the agent learns what it's
being graded against before it edits anything.

## Examples

```json
{ "name": "list_rules", "arguments": {} }
```

## Errors

| MCP error code | Cause |
|---|---|
| `-32603` (internal error) | Failed to load `.sextant/rules/**/*.md` (malformed YAML, IO error). |

## See also

- [`explain_rule`](/sextant-mcp/mcp/tools/explain-rule/) — fetch a
  rule's full markdown.
- [Rules catalog](/sextant-mcp/rules/) — built-in rules.
- [Authoring rules](/sextant-mcp/rules/authoring/) — adding repo
  rules.

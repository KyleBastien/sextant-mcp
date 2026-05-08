---
title: grade_diff
description: Grade only the changed lines since base. Fast — for inner edit loops.
sidebar:
  order: 1
---

Grade only the lines that changed since `base`. Fast — call this in the
inner edit loop after each modification and self-correct before ending
the turn.

Returns a JSON [Report](/sextant-mcp/concepts/report/) with findings,
severity counts, and a verdict.

## Input schema

```json
{
	"type": "object",
	"properties": {
		"base": {
			"type": "string",
			"description": "Base ref. Defaults to merge-base with origin/main, then HEAD~1."
		},
		"head": {
			"type": "string",
			"description": "Head ref. Defaults to working tree."
		},
		"working_tree": {
			"type": "boolean",
			"description": "Force diff against the working tree even if `head` is set."
		}
	}
}
```

All inputs are optional. Calling with `{}` grades the working tree
against the merge-base with `origin/main`.

## Output

A JSON [`Report`](/sextant-mcp/concepts/report/), wrapped in MCP's
`content` envelope:

```json
{
	"content": [
		{
			"type": "text",
			"text": "{ \"summary\": \"...\", \"verdict\": ..., \"counts\": ..., \"findings\": [...] }"
		}
	]
}
```

The text is the JSON-serialized `Report`. Clients should parse it.

## When to call

- After every meaningful edit, in the inner agent loop.
- Right before ending a turn, as a final check.
- The Claude Code plugin's `PostToolUse` and `Stop` hooks call this
  automatically.

When **not** to call it:

- During exploration (no edits yet).
- For docs-only edits in `.md` files where no rules apply.
- After every keystroke — that's what the hook is for.

## Performance

Typically under 500ms. The grade reads only the files in the diff,
runs `file`-scope rules against them, and filters findings to the
changed line ranges. LLM-evaluated rules are cached by content hash —
unchanged files are free.

## Examples

Default — grade the working tree against the merge-base:

```json
{ "name": "grade_diff", "arguments": {} }
```

Grade a specific commit range:

```json
{
	"name": "grade_diff",
	"arguments": {
		"base": "main",
		"head": "feature-branch"
	}
}
```

Grade the working tree even if a stale `--head` ref is set elsewhere:

```json
{
	"name": "grade_diff",
	"arguments": {
		"base": "origin/main",
		"working_tree": true
	}
}
```

## Errors

| MCP error code | Cause |
|---|---|
| `-32602` (invalid params) | Malformed arguments — e.g. `base` is not a string. |
| `-32603` (internal error) | Engine error: the base ref doesn't resolve, the repo isn't a git repo, etc. |

## See also

- [`grade_files`](/sextant-mcp/mcp/tools/grade-files/) — full-file grade.
- [Scopes](/sextant-mcp/concepts/scopes/) — diff vs file vs PR semantics.
- [Report](/sextant-mcp/concepts/report/) — output schema.

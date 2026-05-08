---
title: grade_files
description: Grade entire current contents of given files. Slower, thorough.
sidebar:
  order: 2
---

Grade the entire current contents of the given files (or the whole repo
if `paths` is empty). Slower than `grade_diff`; use for thorough review.

Returns a JSON [Report](/sextant-mcp/concepts/report/).

## Input schema

```json
{
	"type": "object",
	"properties": {
		"paths": {
			"type": "array",
			"items": { "type": "string" },
			"description": "Paths to grade. Empty = whole repo."
		}
	}
}
```

Paths are relative to the repository root. Both files and directories
are accepted; directories are walked respecting `.gitignore`.

## Output

A JSON [`Report`](/sextant-mcp/concepts/report/), wrapped in the MCP
`content` envelope. Identical shape to `grade_diff`, but findings are
not filtered to a diff — every line in the listed files is in scope.

## When to call

- Auditing existing code for debt.
- Reviewing a specific subset of files in detail.
- Confirming a refactor didn't regress neighbouring code (grade the
  refactored file plus its callers).

When **not** to call it:

- In the inner edit loop. Use `grade_diff` instead.
- For "did my last change pass?" — `grade_diff` answers that faster.

## Performance

A few seconds to tens of seconds, depending on repo size and whether
LLM rules are enabled. Diff mode is two to three orders of magnitude
faster on a typical change.

## Examples

Grade the whole repo:

```json
{ "name": "grade_files", "arguments": { "paths": [] } }
```

Grade specific files:

```json
{
	"name": "grade_files",
	"arguments": {
		"paths": ["src/parser.rs", "src/lexer.rs"]
	}
}
```

Grade a directory:

```json
{ "name": "grade_files", "arguments": { "paths": ["src/handlers/"] } }
```

## Errors

| MCP error code | Cause |
|---|---|
| `-32602` (invalid params) | Malformed `paths` argument. |
| `-32603` (internal error) | Path doesn't exist, file isn't UTF-8, engine error. |

## See also

- [`grade_diff`](/sextant-mcp/mcp/tools/grade-diff/) — fast, diff-only.
- [Scopes](/sextant-mcp/concepts/scopes/) — full-file vs diff
  semantics.
- [Report](/sextant-mcp/concepts/report/) — output schema.

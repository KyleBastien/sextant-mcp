---
title: Use with Claude Code
description: Wire sextant-mcp into Claude Code via the bundled plugin or by hand.
sidebar:
  order: 2
---

The fastest way to use Sextant inside [Claude Code](https://claude.ai/code)
is the bundled plugin — it registers the MCP server, three skills,
three slash commands, and a sample git pre-commit hook in one go.

## Recommended: install the plugin

From a Claude Code session:

```text
/plugin marketplace add kylebastien/sextant-mcp
/plugin install sextant@kylebastien/sextant-mcp
```

The plugin lives at `plugin/` in the
[sextant-mcp repo](https://github.com/kylebastien/sextant-mcp), so the
marketplace is the repo itself. After installing, **restart the
session** to pick up the hooks.

See the [Claude Code plugin guide](/sextant-mcp/plugin/) for what each
piece does.

## Manual MCP config

If you want only the MCP server (no hooks, no skills), add it to your
project's `.mcp.json`:

```json
{
	"mcpServers": {
		"sextant": {
			"command": "sextant-mcp"
		}
	}
}
```

Or to your user-level Claude Code config (`~/.claude/mcp.json` on macOS
and Linux). Restart the session after editing.

The server reads config from the current working directory's repo root.
If you run Claude Code from outside a repo, the server starts but
finds no rules.

### With LLM rules

```json
{
	"mcpServers": {
		"sextant": {
			"command": "sextant-mcp",
			"env": {
				"ANTHROPIC_API_KEY": "${ANTHROPIC_API_KEY}"
			}
		}
	}
}
```

Claude Code's MCP env var substitution lets you pass through your
shell's API key without committing it.

## Verify the server is running

In a Claude Code session, ask:

> What MCP tools are available?

You should see `grade_diff`, `grade_files`, `list_rules`,
`explain_rule`, `get_config`. If not, check Claude Code's MCP log
panel — `sextant-mcp` writes its own logs to stderr, which Claude
Code surfaces.

## See also

- [Claude Code plugin](/sextant-mcp/plugin/) — the easy path.
- [Claude Desktop](/sextant-mcp/mcp/claude-desktop/) — same pattern,
  different config path.
- [Tools reference](/sextant-mcp/mcp/) — what the server exposes.

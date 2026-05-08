---
title: Use with Claude Desktop
description: Add sextant-mcp to Claude Desktop's MCP configuration.
sidebar:
  order: 3
---

Claude Desktop discovers MCP servers via its config file. Add Sextant
there once and the tools show up in every conversation.

## 1. Locate the config

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| Windows | `%APPDATA%\Claude\claude_desktop_config.json` |
| Linux | `~/.config/Claude/claude_desktop_config.json` |

Create the file if it doesn't exist.

## 2. Add the server

```json
{
	"mcpServers": {
		"sextant": {
			"command": "sextant-mcp"
		}
	}
}
```

If `sextant-mcp` is not on the global `PATH` (Claude Desktop launches
servers without a login shell), use an absolute path:

```json
{
	"mcpServers": {
		"sextant": {
			"command": "/Users/you/.cargo/bin/sextant-mcp"
		}
	}
}
```

## 3. Wire LLM rules (optional)

If you use rules with `evaluator.type: llm`, pass the API key through
to the server. Claude Desktop doesn't inherit your shell environment,
so the `env` block is the supported way:

```json
{
	"mcpServers": {
		"sextant": {
			"command": "/Users/you/.cargo/bin/sextant-mcp",
			"env": {
				"ANTHROPIC_API_KEY": "sk-ant-api03-..."
			}
		}
	}
}
```

Treat this file as a secret — it sits in plaintext on disk.

## 4. Working directory

Claude Desktop runs MCP servers in your home directory by default. The
server looks for `.sextant/config.toml` in the repo root of its CWD,
so for repo-aware grading you'll want to launch Claude Desktop from
the project root, or set `cwd` explicitly:

```json
{
	"mcpServers": {
		"sextant": {
			"command": "/Users/you/.cargo/bin/sextant-mcp",
			"cwd": "/Users/you/code/your-repo"
		}
	}
}
```

For multi-repo workflows, the [Claude Code plugin](/sextant-mcp/plugin/)
is a better fit — it picks up the repo of whichever session you've
launched.

## 5. Restart Claude Desktop

The MCP config is loaded at startup. Quit and reopen the app to pick up
changes.

## Verify

Ask in a conversation:

> What MCP tools do you have?

You should see all five Sextant tools listed. If not, check Claude
Desktop's developer logs — server stderr is forwarded there.

## See also

- [MCP overview](/sextant-mcp/mcp/) — transports and environment.
- [Tools reference](/sextant-mcp/mcp/) — what each tool does.

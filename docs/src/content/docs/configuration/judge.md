---
title: "[judge]"
description: LLM provider configuration for llm-evaluated rules.
sidebar:
  order: 6
---

`[judge]` configures the LLM provider used by rules with
`evaluator.type: llm`. Without it, LLM rules fail to load.

## Schema

```toml
[judge]
enabled = true
provider = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
max_concurrency = 4
cache_dir = ".sextant/cache/llm"
```

| Field | Type | Default | Effect |
|---|---|---|---|
| `enabled` | bool | `false` | Master on/off. When `false`, all LLM rules are dropped at load time. |
| `provider` | `"anthropic"` \| `"openai"` | `"anthropic"` | Inferred from `model` when omitted. |
| `model` | string | provider-specific | Default model if a rule doesn't override. |
| `api_key_env` | string | `"ANTHROPIC_API_KEY"` / `"OPENAI_API_KEY"` | Env var name to read the API key from. |
| `max_concurrency` | u32 | `4` | Max parallel LLM calls per grade. Tune to your provider's rate limits. |
| `cache_dir` | string | `.sextant/cache/llm` | Where to store BLAKE3-keyed response cache. |

## Why `api_key_env` and not the key itself

Sextant never reads the key from `config.toml` — it reads the **name
of an env var** that holds the key. That keeps the key out of git, out
of CI logs, and out of `cargo` build output. The CLI, MCP server, and
GitHub Action all follow the same convention.

In CI:

```yaml
- uses: kylebastien/sextant-mcp/action@v0.1.0
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

For Claude Desktop / Claude Code, set the env via the MCP server
config:

```json
{
  "mcpServers": {
    "sextant": {
      "command": "sextant-mcp",
      "env": { "ANTHROPIC_API_KEY": "${ANTHROPIC_API_KEY}" }
    }
  }
}
```

## Cache

LLM responses are cached by BLAKE3 hash of `(file content, rule id,
rule body, model)`. Repeat grades of unchanged files are free.

The cache lives in `cache_dir` (default `.sextant/cache/llm`) and is
git-ignored by `sextant init`. Wipe it any time:

```sh
rm -rf .sextant/cache/llm
```

The cache is local-only — CI runs miss it. The PR baseline cache
(see [Baseline cache](/sextant-mcp/action/baseline-cache/)) handles
the CI-side performance story.

## Disabling without removing the section

Two options that compose differently:

- **`enabled = false`** — drop LLM rules at load time. Fastest, no
  network calls.
- **CLI/Action `--no-llm`** — same effect, set per-invocation.

Use `enabled = false` for repos that ship rules but don't want them
running by default; use `--no-llm` for one-off grades or for the
post-edit hook (which uses it by default to keep the inner loop
fast).

## Provider-specific notes

### Anthropic

- Models: `claude-sonnet-4-6`, `claude-opus-4-1`, etc.
- Provider auto-inferred from model name starting with `claude-`.

### OpenAI

- Models: `gpt-4`, `gpt-4o`, `o1-preview`, etc.
- Provider auto-inferred from model name starting with `gpt-` or
  `o1`.

## Cost notes

LLM rules cost real tokens. With concurrency at the default `4`, a
medium-sized repo (50 files in the diff) will issue ~50 LLM calls per
grade. With caching, this is paid once per file change — so the
amortized cost on a steady-state codebase is low. New repos see a
spike on the first PR after enabling LLM rules.

A reasonable starting policy:

- **Local development:** `enabled = true`, accept the cost during
  active editing.
- **CI:** `enabled = true` only on the default branch; `--no-llm` on
  PRs to keep them fast and cheap. Then the only LLM bill is from
  baseline grades on `main`.

## See also

- [Configuration overview](/sextant-mcp/configuration/).
- [Evaluator → llm](/sextant-mcp/concepts/evaluator/#llm--llm-as-judge).
- [Authoring rules](/sextant-mcp/rules/authoring/) — write an LLM
  rule.

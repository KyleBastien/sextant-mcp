---
title: Evaluator
description: The kind of check a rule performs — builtin, regex, ast, or llm.
sidebar:
  order: 6
---

An **evaluator** is the engine behind a rule. It's the component that
takes some code and produces (or doesn't produce) findings. Sextant
ships four evaluator types.

## `builtin` — Rust evaluator

Compiled into the `sextant` binary. Built-ins are the fastest path
because they can use `tree-sitter` ASTs directly and don't need to walk
the rule body.

```yaml
evaluator:
  type: builtin
  name: fn_length
```

The `name` field selects which built-in implementation to use. The
seven shipped built-ins are listed in the
[Rules catalog](/sextant-mcp/rules/).

You **can't author new built-ins from a repo** — that's a contract for
shipped rules in the binary. Anything you write in `.sextant/rules/`
must be `regex` or `llm`.

## `regex` — line-by-line pattern match

The cheapest authoring path. Good for "no `unwrap()` in prod", forbidden
imports, and similar lexical checks.

```yaml
evaluator:
  type: regex
  pattern: '\.unwrap\('
  exclude_paths: ["**/tests/**", "**/*_test.rs"]
```

| Field | Required | Notes |
|---|---|---|
| `pattern` | yes | Standard Rust regex. Matched against each line of each file in scope. |
| `exclude_paths` | no | Glob patterns that skip files. Useful for keeping rules out of test code. |

The rule fires once per matched line. The full body of the rule
(everything below the frontmatter) becomes the `message` of the
finding — keep it focused, since it's what the user reads.

## `ast` — tree-sitter query

Runs a tree-sitter query over the file's parse tree. Strictly more
powerful than `regex` because it sees real syntactic structure: type
positions vs. value positions, types in strings vs. real types,
function signatures vs. call sites.

```yaml
evaluator:
  type: ast
  query: '((predefined_type) @t (#eq? @t "any"))'
  capture: t                          # optional, defaults to first capture
  message: "no `any` allowed"         # optional override
  not_under: [catch_clause]           # optional ancestor-skip
  exclude_paths: ["**/dist/**"]
```

| Field | Required | Notes |
|---|---|---|
| `query` | yes | Tree-sitter query S-expression. Compiled once per language listed in `languages`. |
| `capture` | no | Capture name to anchor the finding line. Defaults to the first capture. |
| `message` | no | Override message. Falls back to `<rule.name>: matched <snippet>`. |
| `not_under` | no | Drop a match if any ancestor's node kind is in this list. Used for context-sensitive exemptions like "allow `unknown` only inside `catch_clause`". |
| `exclude_paths` | no | Glob patterns that skip files. |

The rule must declare at least one entry in `languages:` — the same
query is compiled once per listed language. AST findings are anchored
to the capture's start row, then run through the engine's diff
filter like every other rule output.

`ast` is what powers most [vendor pack
rules](/sextant-mcp/packs/typescript/) where the precision matters:
banning the `any` keyword as a type without flagging the substring
"any" inside a string literal, allowing `as const` while banning all
other `as` casts, etc.

## `llm` — LLM-as-judge

Use the rule body as a prompt. The LLM is asked to find specific
violations in the file and return them as structured findings.

```yaml
evaluator:
  type: llm
  model: claude-sonnet-4-6        # optional; falls back to [judge].model
  max_tokens: 1024                # optional
  temperature: 0.0                # optional
  exclude_paths: ["**/tests/**"]
```

| Field | Required | Notes |
|---|---|---|
| `model` | no | Override `[judge].model`. Provider is inferred from the model name (Claude or GPT). |
| `max_tokens` | no | Per-call cap. |
| `temperature` | no | Defaults to `0.0` for determinism. |
| `exclude_paths` | no | Same as for regex. |

The rule body is the prompt. Placeholders `{{path}}`, `{{code}}`, and
`{{rule.id}}` get substituted at evaluation time. Output is constrained
via tool-use so findings are always well-typed `Finding`s — no JSON
parsing failures, no hallucinated severities.

LLM rules require:

1. A `[judge]` block in `.sextant/config.toml` enabling the provider
   and naming the env var holding the API key.
2. The corresponding API key in the environment (or workflow secret).

LLM-rule responses are cached by content-hash (BLAKE3) under
`.sextant/cache/`, so repeat grades of the same file are free. The
cache is git-ignored.

See [Configuration → judge](/sextant-mcp/configuration/judge/) for the
full provider config.

## Picking an evaluator

| If your rule is… | Use |
|---|---|
| One of the seven built-ins | `builtin` (you wouldn't author this) |
| A simple lexical check that doesn't care about syntax | `regex` |
| A check that needs to distinguish types, function signatures, or other AST shape | `ast` |
| A pattern that needs context, intent, or natural-language reasoning | `llm` |

Default to `regex` for cheap text matches. Reach for `ast` when
false positives in strings or comments are a problem, or when you
need to scope a match to a specific syntactic position. Reserve
`llm` for things the type-system layer can't see.

## See also

- [Authoring rules](/sextant-mcp/rules/authoring/) — full schema.
- [Configuration → judge](/sextant-mcp/configuration/judge/) — wiring
  up an LLM provider.
- [Rule](/sextant-mcp/concepts/rule/) — the metadata around an
  evaluator.

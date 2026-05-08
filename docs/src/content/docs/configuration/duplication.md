---
title: "[duplication]"
description: Token-duplication detection threshold.
sidebar:
  order: 5
---

`[duplication]` tunes the
[`builtin.duplication.tokens`](/sextant-mcp/rules/builtin/duplication-tokens/)
rule.

## Schema

```toml
[duplication]
min_tokens = 100
```

| Field | Default | Effect |
|---|---|---|
| `min_tokens` | `100` | Minimum run length (in tokens) to count as a clone. Lower = more findings. |

A run of 100 tokens is roughly 20 lines of typical code — a comfortable
"could probably be a helper" threshold.

## Tuning

| Use case | `min_tokens` |
|---|---|
| Catch every refactor candidate | `40-60` (noisy) |
| Default | `100` |
| Only flag substantial copy-paste | `200+` |

Lowering increases recall at the cost of precision; raising does the
opposite.

## What "token" means here

The rule walks the tree-sitter token stream and hashes by token
**kind** — the parser's terminal type, not the text. Two snippets are
"the same" if they share the same sequence of kinds, even with
different identifiers and literals.

That makes the rule sensitive to "type-2" clones (renamed copies) but
*also* to incidental structural matches: 100 tokens of
`let x = 1; let y = 2; ...` will match 100 tokens of
`let foo = "a"; let bar = "b"; ...`. Tune `min_tokens` higher if your
codebase's idioms produce false positives.

## Disabling

To turn off the rule entirely, ship a stub override. See
[Authoring rules → override a built-in](/sextant-mcp/rules/authoring/#override-a-built-in).

## See also

- [Configuration overview](/sextant-mcp/configuration/).
- [`builtin.duplication.tokens`](/sextant-mcp/rules/builtin/duplication-tokens/) —
  the rule documentation.

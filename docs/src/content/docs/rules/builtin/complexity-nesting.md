---
title: builtin.complexity.nesting
description: Functions with too many nested control structures.
sidebar:
  label: complexity.nesting
  order: 5
---

| Field | Value |
|---|---|
| **id** | `builtin.complexity.nesting` |
| **severity** | `warn` |
| **category** | `complexity` |
| **scope** | `file` |
| **languages** | rust, python, go, java, typescript, tsx, javascript |
| **evaluator** | `builtin / nesting` |

Deeply nested control flow (`if` inside `for` inside `while` inside
another `if`...) is one of the strongest correlates with bug
density. Each level of nesting multiplies the number of state
combinations the reader has to track and shrinks the working memory
left for the actual logic.

## Thresholds

Configure under `[complexity]` in `.sextant/config.toml`:

```toml
[complexity]
nesting_warn = 4
nesting_error = 6
```

| Setting | Default | Effect |
|---|---|---|
| `nesting_warn` | `4` | Functions reaching this depth trigger a `warn`. |
| `nesting_error` | `6` | Functions reaching this depth escalate to `error`. |

The depth is the maximum nesting level reached anywhere inside the
function body. The function declaration itself counts as level 0.

## Fixing a finding

- **Invert conditions and return early** —
  `if !valid { return Err(_) }` is one level shallower than
  `if valid { ... }`.
- **Extract inner blocks into helpers** — pull the innermost loop
  body into its own function and replace it with a single call.
- **Use iterator chains over manual loops** —
  `iter().filter().map().sum()` is a flat pipeline; the manual
  equivalent is at least two levels of nesting.
- **Split combined predicates** — `if a && b && c` reads better as
  guard clauses; `if (a, b, c) == ...` collapses many branches into
  one match.

## See also

- [`builtin.complexity.cyclomatic`](/sextant-mcp/rules/builtin/complexity-cyclomatic/) —
  related metric on path count.
- [Configuration → complexity](/sextant-mcp/configuration/complexity/).

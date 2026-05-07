---
id: builtin.complexity.nesting
name: "Maximum nesting depth"
description: "Functions with too many nested control structures."
severity: warn
category: complexity
scope: file
languages: [rust, python]
evaluator:
  type: builtin
  name: nesting
enabled: true
tags: [complexity]
---

# Maximum nesting depth

Deeply nested control flow (`if` inside `for` inside `while` inside
another `if`...) is one of the strongest correlates with bug density.
Each level of nesting multiplies the number of state combinations the
reader has to track and shrinks the working memory left for the actual
logic.

## Thresholds

Configure under `[complexity]` in `.sextant/config.toml`:

```toml
[complexity]
nesting_warn = 4
nesting_error = 6
```

## Fixing a finding

- **Invert conditions and return early** — `if !valid { return Err(_) }`
  is one level shallower than `if valid { ... }`.
- **Extract inner blocks into helpers** — pull the innermost loop body
  into its own function and replace it with a single call.
- **Use iterator chains over manual loops** — `iter().filter().map().sum()`
  is a flat pipeline; the manual equivalent is at least two levels of
  nesting.
- **Split combined predicates** — `if a && b && c` reads better as guard
  clauses; `if (a, b, c) == ...` collapses many branches into one match.

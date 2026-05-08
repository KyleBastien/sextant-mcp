---
title: "[complexity]"
description: Thresholds for complexity-category built-in rules.
sidebar:
  order: 4
---

`[complexity]` tunes the thresholds for the two complexity-category
built-ins.

## Schema

```toml
[complexity]
cyclomatic_warn = 10
cyclomatic_error = 20
nesting_warn = 4
nesting_error = 6
```

| Field | Default | Used by |
|---|---|---|
| `cyclomatic_warn` | `10` | [`builtin.complexity.cyclomatic`](/sextant-mcp/rules/builtin/complexity-cyclomatic/) |
| `cyclomatic_error` | `20` | [`builtin.complexity.cyclomatic`](/sextant-mcp/rules/builtin/complexity-cyclomatic/) |
| `nesting_warn` | `4` | [`builtin.complexity.nesting`](/sextant-mcp/rules/builtin/complexity-nesting/) |
| `nesting_error` | `6` | [`builtin.complexity.nesting`](/sextant-mcp/rules/builtin/complexity-nesting/) |

## Calibration

The defaults are conservative. Industry studies typically put
"hard to maintain" at cyclomatic ≥ 10 and "should be split" at ≥ 20.
Nesting depth ≥ 4 starts to hurt readability for most readers.

If your codebase has a lot of pattern-matching (Rust enums, TypeScript
discriminated unions), the cyclomatic rule may be noisier than you'd
like — each `match` arm counts as a path. Two reasonable responses:

1. Raise `cyclomatic_warn` to `15` for a forgiving baseline.
2. Override the rule with an LLM-evaluated alternative that
   distinguishes "exhaustive match" (clear) from "nested branching"
   (complex).

## Disabling individual rules

Same as for size — `[complexity]` has no enable flag. Ship an override
stub. See
[Authoring rules → override a built-in](/sextant-mcp/rules/authoring/#override-a-built-in).

## See also

- [Configuration overview](/sextant-mcp/configuration/).
- [`builtin.complexity.cyclomatic`](/sextant-mcp/rules/builtin/complexity-cyclomatic/).
- [`builtin.complexity.nesting`](/sextant-mcp/rules/builtin/complexity-nesting/).

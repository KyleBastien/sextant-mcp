---
title: builtin.size.fn-length
description: Functions whose body spans more than the configured number of lines.
sidebar:
  label: size.fn-length
  order: 2
---

| Field | Value |
|---|---|
| **id** | `builtin.size.fn-length` |
| **severity** | `warn` |
| **category** | `size` |
| **scope** | `file` |
| **languages** | rust, python, go, java, typescript, tsx, javascript |
| **evaluator** | `builtin / fn_length` |

Long functions tend to bundle multiple responsibilities, hide
branching complexity, and resist isolated testing. A function that
doesn't fit on a screen is a function that's hard to reason about.

## Thresholds

Configure under `[size]` in `.sextant/config.toml`:

```toml
[size]
fn_length_warn = 60
fn_length_error = 120
```

| Setting | Default | Effect |
|---|---|---|
| `fn_length_warn` | `60` | Functions at or above this trigger a `warn`. |
| `fn_length_error` | `120` | Functions at or above this escalate to `error`. |

The line count is from the function's opening brace (or its `def`
line in Python) through the closing brace. Blanks and comments inside
the body count.

## Fixing a finding

- Look for natural seams: a leading "set up" block, a middle "decide"
  block, a trailing "render/persist" block. Each is usually a
  candidate helper.
- Push branches into early returns; pull error mapping into a single
  tail expression.
- If a function manages too much state, consider promoting that state
  into a struct with methods.

## See also

- [`builtin.complexity.cyclomatic`](/sextant-mcp/rules/builtin/complexity-cyclomatic/) —
  often correlated with length.
- [`builtin.size.param-count`](/sextant-mcp/rules/builtin/size-param-count/) —
  long parameter lists are a related smell.
- [Configuration → size](/sextant-mcp/configuration/size/).

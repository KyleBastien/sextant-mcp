---
title: builtin.size.param-count
description: Functions that take more than the configured number of parameters.
sidebar:
  label: size.param-count
  order: 3
---

| Field | Value |
|---|---|
| **id** | `builtin.size.param-count` |
| **severity** | `warn` |
| **category** | `size` |
| **scope** | `file` |
| **languages** | rust, python, go, java, typescript, tsx, javascript |
| **evaluator** | `builtin / param_count` |

Long parameter lists are a smell: usually the function is doing too
much, or the call site is missing an aggregate type that should
obviously exist. They're also a maintenance hazard — every additional
parameter multiplies the number of call sites that need updating
when the contract changes.

## Thresholds

Configure under `[size]` in `.sextant/config.toml`:

```toml
[size]
param_count_warn = 6
param_count_error = 10
```

| Setting | Default | Effect |
|---|---|---|
| `param_count_warn` | `6` | Functions with this many parameters trigger a `warn`. |
| `param_count_error` | `10` | Functions with this many escalate to `error`. |

The count is over declared parameters. `&self` / `&mut self` /
`self` count. Default arguments count.

## Fixing a finding

- Group related arguments into a struct (`Config`, `Options`, request
  type). Two arguments that always travel together are already a
  struct in disguise.
- Promote optional parameters into a builder if there are more than a
  few.
- For methods with many `&mut self` arguments plus extras, consider
  whether some of those extras belong as fields on `Self`.

## See also

- [`builtin.size.fn-length`](/sextant-mcp/rules/builtin/size-fn-length/) —
  often co-occurs with high param count.
- [Configuration → size](/sextant-mcp/configuration/size/).

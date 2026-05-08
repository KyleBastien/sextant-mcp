---
title: "[size]"
description: Thresholds for size-category built-in rules.
sidebar:
  order: 3
---

`[size]` tunes the thresholds for the three size-category built-ins.

## Schema

```toml
[size]
file_length_warn = 400
file_length_error = 800
fn_length_warn = 60
fn_length_error = 120
param_count_warn = 6
param_count_error = 10
```

| Field | Default | Used by |
|---|---|---|
| `file_length_warn` | `400` | [`builtin.size.file-length`](/sextant-mcp/rules/builtin/size-file-length/) |
| `file_length_error` | `800` | [`builtin.size.file-length`](/sextant-mcp/rules/builtin/size-file-length/) |
| `fn_length_warn` | `60` | [`builtin.size.fn-length`](/sextant-mcp/rules/builtin/size-fn-length/) |
| `fn_length_error` | `120` | [`builtin.size.fn-length`](/sextant-mcp/rules/builtin/size-fn-length/) |
| `param_count_warn` | `6` | [`builtin.size.param-count`](/sextant-mcp/rules/builtin/size-param-count/) |
| `param_count_error` | `10` | [`builtin.size.param-count`](/sextant-mcp/rules/builtin/size-param-count/) |

Each `_warn` value is the lower bound for a `warn` finding; `_error`
is the bound for `error`. A finding always fires at the highest
applicable severity — a 900-line file fires once at `error`, not
twice.

## Calibration

Reasonable starting points:

| Codebase style | `fn_length_warn` | `fn_length_error` |
|---|---|---|
| Strict (Rust core libs, hot paths) | `30` | `60` |
| Default | `60` | `120` |
| Lenient (legacy, large procedural code) | `100` | `200` |

Files are similar — Rust modules tend to land at 200-400 lines, but
TypeScript components can balloon to 600+ before being unwieldy.
Calibrate to your codebase's conventions; the defaults err on the
conservative side.

## Disabling individual rules

`[size]` doesn't have an enable/disable flag — to turn off
`builtin.size.fn-length` entirely, ship a stub override:

```yaml
# .sextant/rules/disable-fn-length.md
---
id: builtin.size.fn-length
name: "(disabled)"
description: "x"
severity: info
category: size
enabled: false
evaluator: { type: regex, pattern: "(?!)" }
---
```

See
[Authoring rules → override a built-in](/sextant-mcp/rules/authoring/#override-a-built-in).

## See also

- [Configuration overview](/sextant-mcp/configuration/) — full
  schema.
- The three size rules:
  [file-length](/sextant-mcp/rules/builtin/size-file-length/),
  [fn-length](/sextant-mcp/rules/builtin/size-fn-length/),
  [param-count](/sextant-mcp/rules/builtin/size-param-count/).

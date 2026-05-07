---
id: builtin.size.fn-length
name: "Function length"
description: "Functions whose body spans more than the configured number of lines."
severity: warn
category: size
scope: file
languages: [rust]
evaluator:
  type: builtin
  name: fn_length
enabled: true
tags: [size, complexity]
---

# Function length

Long functions tend to bundle multiple responsibilities, hide branching
complexity, and resist isolated testing. A function that doesn't fit on a
screen is a function that's hard to reason about.

## Thresholds

Configure under `[size]` in `.sextant/config.toml`:

```toml
[size]
fn_length_warn = 60
fn_length_error = 120
```

## Fixing a finding

- Look for natural seams: a leading "set up" block, a middle "decide" block,
  a trailing "render/persist" block. Each is usually a candidate helper.
- Push branches into early returns; pull error mapping into a single tail
  expression.
- If a function manages too much state, consider promoting that state into
  a struct with methods.

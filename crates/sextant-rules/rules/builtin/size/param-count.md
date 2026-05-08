---
id: builtin.size.param-count
name: "Parameter count"
description: "Functions that take more than the configured number of parameters."
severity: warn
category: size
scope: file
languages: [rust, python, go, java, typescript, tsx, javascript]
evaluator:
  type: builtin
  name: param_count
enabled: true
tags: [size, api]
---

# Parameter count

Long parameter lists are a smell: usually the function is doing too much,
or the call site is missing an aggregate type that should obviously exist.
They're also a maintenance hazard — every additional parameter multiplies
the number of call sites that need updating when the contract changes.

## Thresholds

Configure under `[size]` in `.sextant/config.toml`:

```toml
[size]
param_count_warn = 6
param_count_error = 10
```

## Fixing a finding

- Group related arguments into a struct (`Config`, `Options`, request
  type). Two arguments that always travel together are already a struct
  in disguise.
- Promote optional parameters into a builder if there are more than a
  few.
- For methods with many `&mut self` arguments plus extras, consider
  whether some of those extras belong as fields on `Self`.

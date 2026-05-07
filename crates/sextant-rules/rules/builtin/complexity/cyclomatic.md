---
id: builtin.complexity.cyclomatic
name: "Cyclomatic complexity"
description: "Functions with too many independent control-flow paths."
severity: warn
category: complexity
scope: file
languages: [rust, python]
evaluator:
  type: builtin
  name: cyclomatic
enabled: true
tags: [complexity]
---

# Cyclomatic complexity

Cyclomatic complexity measures the number of linearly independent paths
through a function — `1` for a straight-line function, plus one for every
branch (`if`, `match`/`switch` arm, `while`, `for`, `try`/`except`,
ternary, etc.). It's a strong predictor of how hard a function is to test
and maintain: every additional path is another case the reader must hold
in their head and another scenario the tests must cover.

## Thresholds

Configure under `[complexity]` in `.sextant/config.toml`:

```toml
[complexity]
cyclomatic_warn = 10
cyclomatic_error = 20
```

## Fixing a finding

- **Extract guard clauses** — turn early-return branches into top-of-function
  validation so the body deals only with the happy path.
- **Pull conditional logic into helpers** — a `match` over many cases is
  often clearer when each arm calls a named function.
- **Replace flag arguments with separate functions** — `do_thing(opts)` that
  branches on `opts.kind` is usually two functions in disguise.
- **Use polymorphism / table-driven dispatch** — long `if/else if` chains
  on a tag value beg for a lookup table or trait object.

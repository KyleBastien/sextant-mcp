---
id: vendor.typescript.no-eval
name: "No `eval()` calls"
description: "Bans `eval()`. It's a security and performance disaster, and there's always a better tool."
severity: error
category: security
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '(call_expression function: (identifier) @f (#eq? @f "eval"))'
  capture: f
  message: "no `eval()` — use a parser, a function reference, or `JSON.parse` instead"
tags: [strict, security]
---

# No `eval()` calls

`eval()` runs arbitrary string-as-code in the current scope. It defeats
every static analysis tool, blocks JIT optimizations, and is the
canonical injection vector when its input is anything you didn't
literally write yourself.

**Do this instead:**

- Parsing JSON: `JSON.parse(input)` (with a schema for trust).
- Loading a function by name: keep a `Record<string, Fn>` map.
- Computing math from a string: use a real expression parser
  (`mathjs`, `expr-eval`, etc.).
- Templating: a real templating library, not string concatenation
  feeding into `eval`.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

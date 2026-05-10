---
id: vendor.typescript.no-var
name: "No `var` declarations"
description: "Bans `var`. Use `const` (default) or `let` (when reassignment is needed)."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '(variable_declaration) @v'
  capture: v
  message: "no `var` — use `const` (default) or `let`"
tags: [strict, hoisting]
---

# No `var` declarations

`var` is function-scoped and hoisted, two behaviors that are almost
never what the author intends. `const` and `let` are block-scoped and
not hoisted into the temporal dead zone in confusing ways.

**Do this instead:**

- Default to `const`. If the value never reassigns, the binding never
  reassigns either — that's the most precise expression of intent.
- Use `let` only when reassignment is part of the algorithm.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

---
id: vendor.typescript.no-type-assertion
name: "No angle-bracket type assertion"
description: "Bans the `<Foo>x` type-assertion syntax. Use type narrowing instead."
severity: error
category: reliability
scope: file
languages: [typescript]
evaluator:
  type: ast
  query: '(type_assertion) @cast'
  capture: cast
  message: "no `<Type>x` casts — narrow the type instead, or parse it at the boundary"
tags: [strict, types]
---

# No angle-bracket type assertion

The `<Foo>x` syntax does the same thing as `x as Foo` — both are
unchecked type assertions. The angle-bracket form has the additional
problem that it conflicts with JSX, which is why it's banned in `.tsx`
files (and why this rule scopes to `[typescript]` only).

Use type narrowing the same way as for `as`-casts; see
`vendor.typescript.no-as-cast` for the recipe.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

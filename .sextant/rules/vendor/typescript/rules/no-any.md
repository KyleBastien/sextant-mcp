---
id: vendor.typescript.no-any
name: "No `any` type"
description: "Bans the `any` type in any type position. Use a generic, a precise type, or `unknown` inside a `catch` clause."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((predefined_type) @t (#eq? @t "any"))'
  capture: t
  message: "no `any` allowed — use a generic or a precise type"
tags: [strict, types]
---

# No `any` type

`any` opts a value out of the type system, making the surrounding code
impossible to refactor safely and silently propagating bugs into callers.

**Do this instead:**

- For genuinely polymorphic code, take a generic parameter:
  ```ts
  function pick<T>(items: T[], i: number): T { return items[i]; }
  ```
- For data crossing a trust boundary (parsing JSON, reading user input),
  type the input as `unknown` *only* inside a `catch (e: unknown)` clause —
  elsewhere, narrow with a type guard or a schema (zod, valibot) and assign
  the validated value to its real type.
- If you need the value to look distinct from a structurally-identical one,
  give it a tagged shape (`{ kind: "user"; id: string }`) or wrap it in a
  class. Branded `unique symbol` types are also banned — see
  `vendor.typescript.no-branded-types`.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file. If you find a case where the rule is wrong, the right fix is to
make the rule smarter, not to turn it off.

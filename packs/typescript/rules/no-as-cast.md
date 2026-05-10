---
id: vendor.typescript.no-as-cast
name: "No `as` cast"
description: "Bans the `as` cast (`x as Foo`). Allows `as const`. Use type narrowing or a parser at the boundary."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '(as_expression (_) (_) @t) @cast'
  capture: cast
  message: "no `as` casts — narrow the type instead, or parse it at the boundary"
tags: [strict, types]
---

# No `as` cast

`x as Foo` is a type assertion: it tells the compiler "trust me, this
is a `Foo`" — and the compiler does, with no runtime check. When the
assertion is wrong, the bug surfaces somewhere downstream as a confusing
runtime error.

Allowed: `as const`. The `as const` assertion narrows literal types
(`"hello"` rather than `string`) and is the opposite of casting away
type information.

**Do this instead:**

- Narrow with a type guard:
  ```ts
  if (typeof value === "string") {
    // value is `string` here, no cast needed
  }
  ```
- Use `instanceof` for class hierarchies.
- For untrusted input (JSON, network, IPC), parse at the boundary with
  zod / valibot / a hand-written guard, and let the compiler infer the
  validated type.
- For tagged unions, use the discriminator field directly:
  ```ts
  type Result = { ok: true; value: T } | { ok: false; error: E };
  if (r.ok) { /* r.value is T */ }
  ```

**Cannot be disabled:** the lock-integrity check rejects edits to this
file. Reach for narrowing first; if you genuinely need a cast, re-think
the upstream type.

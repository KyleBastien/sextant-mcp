---
id: vendor.typescript.no-unknown
name: "No `unknown` outside catch"
description: "Bans the `unknown` type outside of a `catch (e: unknown)` clause. Use a generic instead."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((predefined_type) @t (#eq? @t "unknown"))'
  capture: t
  not_under: [catch_clause]
  message: "no `unknown` here — use a generic; reserve `unknown` for `catch (e: unknown)`"
tags: [strict, types]
---

# No `unknown` outside catch

`unknown` is the "I do not know the type" type, and it earns its keep in
exactly one place: `catch (e: unknown) { ... }`, where the language
itself can't promise you a specific exception shape.

Everywhere else, `unknown` is a code smell — it almost always means the
author wanted a generic but reached for the looser tool. The downstream
cost is that every consumer has to narrow the value before doing
anything with it.

**Do this instead:**

- Take a generic parameter:
  ```ts
  function first<T>(xs: readonly T[]): T | undefined { return xs[0]; }
  ```
- For data from untrusted sources, parse it into a real type at the
  boundary (zod / valibot / a hand-written guard) and pass the typed
  value through the rest of your code.

**Allowed:** `try { ... } catch (e: unknown) { ... }` — the rule's
`not_under: [catch_clause]` exemption skips matches whose ancestor
includes a `catch_clause` AST node. Inside the catch body itself, you
should narrow `e` with `instanceof` / `typeof` checks before using it.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file. If you think the rule is firing wrongly, the fix is to refine the
exemption mechanism, not to turn it off.

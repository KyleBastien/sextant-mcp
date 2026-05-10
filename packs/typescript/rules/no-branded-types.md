---
id: vendor.typescript.no-branded-types
name: "No branded types"
description: "Bans `unique symbol` types — the canonical mechanism for nominal/branded types. Define a real strong type instead."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((predefined_type) @t (#match? @t "^unique\\s+symbol$"))'
  capture: t
  message: "no branded types — define a strong type with the right shape, not a `unique symbol` brand"
tags: [strict, types]
---

# No branded types

Branded types — also called nominal types — are an escape hatch for
when the structural type system disagrees with the author's intent.
The canonical pattern is:

```ts
// Banned
type UserId = string & { readonly __brand: unique symbol };
type Order = { id: number; readonly _tag: unique symbol };
const FOO: unique symbol = Symbol("foo");
```

The rule fires on any use of `unique symbol`, which is the only type
expression for branded values. If you find yourself reaching for a
brand, the type you actually want is more specific:

**Do this instead:**

- Wrap the value in a tagged record:
  ```ts
  type UserId = { kind: "user"; id: string };
  type OrderId = { kind: "order"; id: string };
  ```
  TypeScript distinguishes these structurally because `kind` is a
  literal type — no brand needed.
- Use a class, which has nominal identity for free:
  ```ts
  class UserId { constructor(public readonly value: string) {} }
  ```
- Validate input at the boundary and then trust the resulting type.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

---
id: vendor.typescript.no-empty-type-construction
name: "No empty-type construction"
description: "Bans `Pick<T, never>`, `Record<never, V>`, and `Omit<T, keyof T>` constructions that resolve to `{}`."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: |
    (generic_type
      (type_identifier) @name
      (type_arguments (_) (predefined_type) @arg)
      (#eq? @name "Pick") (#eq? @arg "never")) @g
    (generic_type
      (type_identifier) @name
      (type_arguments (predefined_type) @arg (_))
      (#eq? @name "Record") (#eq? @arg "never")) @g
    (generic_type
      (type_identifier) @name
      (type_arguments (_) (index_type_query))
      (#eq? @name "Omit")) @g
  capture: g
  message: "no empty-type construction — this resolves to `{}`"
tags: [strict, types]
---

# No empty-type construction

These constructions all resolve to the empty object type `{}`:

```ts
// Banned
type E = Pick<User, never>;
type R = Record<never, string>;
type O = Omit<User, keyof User>;
```

`{}` in TypeScript means "any non-nullish value" — it accepts numbers,
strings, arrays, functions, anything that isn't `null` or `undefined`.
That's almost never what the author intended, and it slips past
[`vendor.typescript.no-empty-object-type`](./no-empty-object-type.md)
because the *literal* `{}` token never appears in the source.

The `keyof T` form is especially insidious: it looks like a refactor-
safe way of saying "drop all keys." It is — and the result is `{}`,
which then accepts anything.

**Do this instead:**

- If you want a specific subset of keys, name them:
  ```ts
  type Public = Pick<User, "id" | "email" | "displayName">;
  ```
- If you want a real map, give `Record` a sensible key type:
  ```ts
  type Cache = Record<string, User>;
  ```
- If you want "no fields," ask yourself why you're constructing a type
  at all. Often the answer is to delete the type and pass nothing,
  or to use a tagged variant of a discriminated union.

The rule fires on:

- `Pick<T, never>` — second arg is the literal type `never`.
- `Record<never, V>` — first arg is the literal type `never`.
- `Omit<T, keyof X>` — second arg is a `keyof` expression. The rule
  doesn't check that `T === X` (tree-sitter predicates can't compare
  captures structurally), so it also flags `Omit<A, keyof B>` with
  mismatched generics. In practice that pattern is also wrong.

**Cannot be disabled:** the lock-integrity check rejects edits to
this file.

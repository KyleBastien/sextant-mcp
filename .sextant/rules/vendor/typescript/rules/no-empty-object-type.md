---
id: vendor.typescript.no-empty-object-type
name: "No `{}` object type"
description: "Bans `{}` as a type. It means \"anything except `null` or `undefined`\" — almost certainly not what you meant."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((object_type) @b (#match? @b "^\\{\\s*\\}$"))'
  capture: b
  message: "no `{}` type — describe the actual shape or use a precise type"
tags: [strict, types]
---

# No `{}` object type

`{}` as a type is one of TypeScript's biggest footguns. It looks like
"empty object" but actually means "any non-nullish value" — every
primitive, every function, every array, and every object will assign
to it. It's effectively a less-honest `unknown` (which is itself only
permitted inside `catch (e: unknown)`).

```ts
// Banned
type Bag = {};
function configure(opts: {}) { /* ... */ }
const x: {} = "I am a string, not an object";   // type-checks!
```

This rule fires on any `{}` in type position. The companion
`no-empty-interface` rule covers the `interface Foo {}` form.

**Do this instead:**

- Describe the actual shape:
  ```ts
  type User = { id: string; email: string };
  ```
- For a map with arbitrary keys and a *named* value type, use `Record`
  (a primitive-valued `Record<string, string>` is itself banned — see
  `vendor.typescript.no-property-bags`):
  ```ts
  type UsersById = Record<string, User>;
  ```
- For "must be passed but ignored" (rare), make the type explicit
  about what you'll do with it.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

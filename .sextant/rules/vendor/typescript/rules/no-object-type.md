---
id: vendor.typescript.no-object-type
name: "No `object` type"
description: "Bans the lowercase `object` type. Use a precise interface or `Record<string, T>`."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((predefined_type) @t (#eq? @t "object"))'
  capture: t
  message: "no `object` type — describe the shape with an interface or use `Record<string, T>`"
tags: [strict, types]
---

# No `object` type

The lowercase `object` type means "any non-primitive value." Like `any`,
it's a signal that the author didn't yet know what shape the value has
— and like `any`, that uncertainty leaks into every consumer.

**Do this instead:**

- Describe the actual shape with an interface or type alias:
  ```ts
  type User = { id: string; email: string };
  ```
- For dynamic key/value maps, use `Record`:
  ```ts
  type Headers = Record<string, string>;
  ```
- For "either an object or null" (e.g. a function that may return
  nothing structured), spell out the union explicitly.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

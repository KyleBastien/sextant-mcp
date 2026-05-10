---
id: vendor.typescript.no-empty-interface
name: "No empty interface"
description: "Bans `interface Foo {}` with no members. Use a type alias or add real members."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '(interface_declaration body: (interface_body) @body (#match? @body "^\\{\\s*\\}$")) @i'
  capture: i
  message: "no empty interface — add members or remove the declaration"
tags: [strict, types]
---

# No empty interface

`interface Foo {}` is the union of all object types — equivalent to
`{}`, which itself is "anything except `null` and `undefined`." It's
almost never what the author meant.

**Do this instead:**

- If you wanted a type alias for an external shape, use `type Foo = ...`
  with the real shape spelled out.
- If you wanted a marker / brand, use the `unique symbol` pattern:
  ```ts
  type UserId = string & { readonly __brand: unique symbol };
  ```
- If you intended to extend an existing type, do that:
  ```ts
  interface Admin extends User { readonly role: "admin" }
  ```

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

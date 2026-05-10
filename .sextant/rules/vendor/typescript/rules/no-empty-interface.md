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

`interface Foo {}` is equivalent to the bare `{}` type — "anything
except `null` and `undefined`." It's almost never what the author
meant. The companion `no-empty-object-type` rule catches the same
shape outside of an `interface` declaration.

**Do this instead:**

- If you wanted a type alias for an external shape, use `type Foo = ...`
  with the real shape spelled out.
- If you wanted to distinguish two structurally-identical values, give
  them a tagged shape (`{ kind: "user"; id: string }`) or wrap them in
  classes. Branded `unique symbol` types are also banned — see
  `vendor.typescript.no-branded-types`.
- If you intended to extend an existing type, do that:
  ```ts
  interface Admin extends User { readonly role: "admin" }
  ```

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

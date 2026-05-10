---
id: vendor.typescript.no-function-type
name: "No `Function` type"
description: "Bans the global `Function` type. Use a specific call signature."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((type_identifier) @t (#eq? @t "Function"))'
  capture: t
  message: "no `Function` type — use a specific call signature like `(x: T) => U`"
tags: [strict, types]
---

# No `Function` type

The global `Function` type accepts any callable with any signature and
returns `any`. Like `any` itself, it removes the type-checker from the
loop precisely where it would catch the most bugs.

**Do this instead:** spell out the call signature you actually accept:

```ts
// Yes
type Listener = (event: MouseEvent) => void;
type Mapper<T, U> = (input: T) => U;

// No
type Listener = Function;
```

If you genuinely don't care about the signature (rare — almost always
the call site DOES care), `() => unknown` is a more honest "any
zero-arg function returning something."

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

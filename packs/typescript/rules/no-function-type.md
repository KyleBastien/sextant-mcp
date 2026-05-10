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

If you genuinely don't care about the signature, you almost certainly
do — figure out which arguments your code passes and what the return
value is used for, and write that signature. `unknown` is also banned
outside `catch` (see `vendor.typescript.no-unknown`), so reaching for
`() => unknown` isn't an out either.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

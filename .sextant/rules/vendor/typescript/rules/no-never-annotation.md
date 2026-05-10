---
id: vendor.typescript.no-never-annotation
name: "No `never` annotation"
description: "Bans `: never` type annotations outside of conditional types. Throw an error or model unreachability with an exhaustive switch."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((predefined_type) @t (#eq? @t "never"))'
  capture: t
  not_under: [conditional_type]
  message: "no `: never` annotation — narrow exhaustively, do not smuggle"
tags: [strict, types]
---

# No `: never` annotation

`never` is the type with no inhabitants. Putting `: never` on a value
position is a way of asserting "this code path is unreachable" without
actually proving it — and in practice it almost always smuggles a real
value past the type system, paired with a cast.

```ts
// Banned
function load(id: string): never {
  return cache.get(id) as never;
}
const x: never = doThing();
```

The compiler can't argue with `never` once you've claimed it; the cast
turns the rest of the program into a lie. When the call returns
normally — and one day it will — the consumer sees a runtime value at
a type that says "I cannot exist."

**Do this instead:**

- If a function genuinely never returns, throw and let return-type
  inference handle it. The inferred type is already `never`:
  ```ts
  function unreachable(msg: string): never {
    throw new Error(`unreachable: ${msg}`);
  }
  ```
- If you want exhaustiveness over a discriminated union, use the
  exhaustiveness pattern in a conditional or switch — `: never` is
  legitimate **inside a conditional type** (`T extends X ? Y : never`),
  which the rule allows:
  ```ts
  type NonNull<T> = T extends null | undefined ? never : T;
  ```
- For "shouldn't reach this branch" assertions, write a real helper
  whose argument is `never`. The compile-time error fires when a new
  variant is added:
  ```ts
  function assertNever(_: never): never { throw new Error("missed case"); }
  switch (kind) {
    case "a": ...;
    case "b": ...;
    default: assertNever(kind);
  }
  ```

**Allowed:** any `never` reached *under* a `conditional_type` AST
node. The rule's `not_under: [conditional_type]` exemption skips
matches whose ancestor includes that node.

**Cannot be disabled:** the lock-integrity check rejects edits to
this file.

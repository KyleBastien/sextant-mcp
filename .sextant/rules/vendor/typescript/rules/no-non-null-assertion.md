---
id: vendor.typescript.no-non-null-assertion
name: "No `!` non-null assertion"
description: "Bans the `x!` non-null assertion. Use a type guard to narrow `null`/`undefined` away."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '(non_null_expression) @e'
  capture: e
  message: "no `!` non-null assertion — narrow with a type guard"
tags: [strict, types]
---

# No `!` non-null assertion

`x!` tells the compiler "this is definitely not null or undefined" with
zero runtime check. When you're wrong, you get a `TypeError: Cannot
read properties of undefined` at the next access.

**Do this instead:**

- Narrow explicitly:
  ```ts
  if (x === undefined) throw new Error("...");
  // x is non-undefined here
  ```
- Or use the `??` / `?.` operators:
  ```ts
  const value = config.title ?? "default";
  ```
- For arrays and maps, use methods that return non-undefined when
  you've already verified non-emptiness, or use a real `unwrap`
  helper that throws with a useful message.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

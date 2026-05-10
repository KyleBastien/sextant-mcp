---
id: vendor.typescript.no-ts-ignore
name: "No `@ts-ignore` directives"
description: "Bans `// @ts-ignore`, `// @ts-expect-error`, and `// @ts-nocheck` comments. Fix the underlying type error."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((comment) @c (#match? @c "@ts-(ignore|expect-error|nocheck)\\b"))'
  capture: c
  message: "no `@ts-ignore` / `@ts-expect-error` / `@ts-nocheck` — fix the underlying type error"
tags: [strict, suppressions]
---

# No `@ts-(ignore|expect-error|nocheck)` directives

These comments tell the TypeScript compiler "skip type checking here."
That's never the right answer:

- `// @ts-ignore` — silently suppresses the next line. Errors that
  surface later (after a refactor changes types) will still be
  suppressed, hiding real bugs.
- `// @ts-expect-error` — supposedly self-cleaning, but in practice
  agents and humans alike forget to remove it once the underlying error
  is fixed.
- `// @ts-nocheck` — disables type checking for the entire file. Use
  this and you've effectively converted the file to JavaScript.

**Do this instead:** fix the underlying type error. If the type system
is wrong about your code, the fix is to express the type more precisely
(generics, type guards, tagged unions) — not to silence the compiler.
Branded / `unique symbol` types are also banned (see
`vendor.typescript.no-branded-types`), so use a tagged record or a
class when you need nominal identity.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

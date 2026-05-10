---
id: vendor.typescript.prefer-inferred-types
name: "Prefer inferred types over redundant annotations"
description: "Flags `const x: string = \"hi\"` where the annotation is redundant — let TypeScript infer."
severity: error
category: style
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: |
    (variable_declarator
      type: (type_annotation (predefined_type) @t)
      [(string) (number) (true) (false) (template_string)] @v) @decl
  capture: decl
  message: "drop the type annotation — TypeScript will infer the same type from the initializer"
tags: [strict, style]
---

# Prefer inferred types

When a `const` or `let` declaration is initialized with a primitive
literal, the type the compiler would infer is identical to the one
you spelled out. The annotation is noise.

```ts
// Annotation is redundant
const greeting: string = "hello";
let count: number = 0;
const enabled: boolean = true;

// Let TypeScript infer
const greeting = "hello";   // string
let count = 0;              // number
const enabled = true;       // boolean
```

The literal-and-redundant pattern is the most common case — but the
underlying principle generalizes: trust the inference engine when the
right-hand side already pins down the type.

**Why this matters more for agents than for humans:** an agent's reflex
is often to "be explicit" by adding a type annotation, even when doing
so locks the binding into an unnecessarily wide type. `const x = "foo"`
gives `"foo"`; `const x: string = "foo"` gives `string`. Inference is
strictly more informative here.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file.

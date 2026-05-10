---
id: vendor.typescript.no-ambient-module-shim
name: "No empty `declare module` shim"
description: "Bans `declare module \"x\" {}` declarations with an empty body. These make every import from the module resolve to `any`."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: regex
  pattern: "^\\s*declare\\s+module\\s+[\"'][^\"']+[\"']\\s*\\{\\s*\\}\\s*;?\\s*$"
  replacement: ""
tags: [strict, types]
---

# No empty `declare module` shim

```ts
// Banned
declare module "untyped-pkg" {}
declare module "*.svg" {}
```

A `declare module "..."` block with an empty body tells TypeScript
"this module exists but I'm not going to describe its shape." Every
import from that path then resolves to `any`, silently — without a
single `any` keyword in your code.

```ts
import { load } from "untyped-pkg"; // load: any
load(123).foo.bar.baz;              // also any, no error
```

The `*` wildcard form (`declare module "*.svg" {}`) makes this worse:
*every* untyped import that matches the pattern becomes `any`. One
two-line file silently disables type-checking across an entire class
of imports.

**Do this instead:**

- Install community types: `npm i -D @types/the-package`. Most popular
  libraries have them.
- Write real ambient types in a `.d.ts` file. Even rough signatures
  (`export function load(path: string): Buffer;`) catch most bugs:
  ```ts
  // types/untyped-pkg.d.ts
  declare module "untyped-pkg" {
    export function load(path: string): Buffer;
    export const VERSION: string;
  }
  ```
- For asset imports (`*.svg`, `*.png`), import-types ship with most
  bundler integrations (Vite, webpack-loader presets) — adopt the
  bundler's preset rather than rolling your own shim.

The proposed autofix removes the empty-shim line; the author writes
real ambient types in its place.

**Allowed:** populated `declare module "x" { … }` declarations are a
legitimate augmentation pattern and don't match the rule.

**Cannot be disabled:** the lock-integrity check rejects edits to
this file.

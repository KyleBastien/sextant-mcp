---
id: vendor.typescript.no-jsdoc-types
name: "No JSDoc type annotations"
description: "Bans JSDoc type comments (`@type {…}`, `@param {…}`, `@returns {…}`, etc.) in `.ts` / `.tsx` files. Write a real TypeScript annotation."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: regex
  pattern: '@(type|param|returns?|typedef|property)\s*\{[^}]*\}'
  replacement: '@$1'
tags: [strict, suppressions]
---

# No JSDoc type annotations in `.ts` / `.tsx`

JSDoc type tags (`@type {string}`, `@param {number} n`, `@returns
{Promise<User>}`, `@typedef`, `@property`) are a JavaScript-era
mechanism for telling the type-checker about an untyped value. They
survive a `.js` → `.ts` rename and quietly continue to be the
load-bearing type information for the file — except now you have a
TypeScript file whose types live in comments.

```ts
// Banned
/** @type {string} */
const x = "hi";
/** @param {number} n - count */
/** @returns {boolean} */
function isEven(n) { return n % 2 === 0; }
```

**Do this instead** — write a real TypeScript annotation in the type
position. JSDoc descriptions without the brace form are fine and stay
useful for tools and humans:

```ts
const x: string = "hi";

/** Count of items. */
function isEven(n: number): boolean { return n % 2 === 0; }
```

The proposed autofix strips the `{…}` payload and leaves the bare
tag (`@type` becomes `@type`, `@param {number} n` becomes
`@param n`); the author's job is to add the equivalent TypeScript
annotation on the following declaration.

**Cannot be disabled:** the lock-integrity check rejects edits to
this file.

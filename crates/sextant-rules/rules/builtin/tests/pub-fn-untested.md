---
id: builtin.tests.pub-fn-untested
name: "Public function without a test"
description: "Public Rust functions or exported JS/TS declarations whose name is not mentioned in any in-file or peer-file test."
severity: info
category: tests
scope: file
languages: [rust, javascript, typescript, tsx]
evaluator:
  type: builtin
  name: pub_fn_untested
enabled: true
tags: [tests, coverage]
---

# Public function without a test

Flags public-API definitions whose name does not appear in any test
body — checked first in the source file itself, then in a small set of
conventional peer test files sitting next to (or near) the source.
Severity is `info`: a signal to help focus attention, not a
verdict-breaker.

The rule prefers tests in a peer file — that is the more common
layout across Rust crates and JS/TS projects — but an in-file
`#[cfg(test)] mod` (Rust) or in-source Vitest block (JS/TS) is also
accepted. Both shapes silence the finding.

## Where the rule looks for tests

### Rust

For source file `<dir>/<stem>.rs`, the rule considers:

1. In-file `#[test]`, `#[tokio::test]`, and any `…::test`-attributed
   function bodies (including helpers inside a `#[cfg(test)] mod` —
   those bodies are pulled into the haystack too).
2. `<dir>/<stem>_tests.rs` — the sibling-file convention used by this
   workspace and many crates that want tests next to the code without
   inflating the source file's line count.
3. `<dir>/tests/<stem>.rs` — a `tests/` directory next to the source.
4. `<crate-root>/tests/<stem>.rs` — Cargo's integration-test directory
   at the crate root (found by walking up to the parent of `src/`).

### JavaScript / TypeScript / TSX

For source file `<dir>/<stem>.<ext>`, the rule considers:

1. In-file `describe` / `it` / `test` / `suite` / `context` / `fit` /
   `fdescribe` callback bodies — including chained forms like
   `it.skip(…)`, `describe.only(…)`, and `test.each(table)(name, fn)`.
   This covers Jest, Vitest, Mocha, and Jasmine in-source patterns.
2. Sibling `<dir>/<stem>.test.<ext>` or `<dir>/<stem>.spec.<ext>`
   (across `ts`, `tsx`, `js`, `jsx`, `mjs`, `cjs`).
3. The same two shapes nested under a `__tests__/` directory beside
   the source.

Matching is a whole-word identifier match against the file's text. The
peer-file haystack is the raw file contents — no parsing — which keeps
the check cheap and language-agnostic.

## Per-language definitions

### Rust

- Only fully-public `pub fn`. `pub(crate)`, `pub(super)`, and
  `pub(in …)` are excluded — they're internal API.
- Functions inside a module annotated with `#[cfg(test)]` are excluded
  (they're test helpers, not public surface).

### JavaScript / TypeScript / TSX

- Only declarations introduced with the `export` keyword are
  considered public. Bare top-level `function f` and `const f = …`
  are module-private and skipped (analogous to Rust without `pub`).
- Covered shapes: `export function f`, `export async function f`,
  `export function* f`, `export const f = (…) => …`,
  `export const f = function (…) {…}`, `export class C`, and
  `export default function f` (with a name). Anonymous default
  exports (`export default () => …`) and re-export lists
  (`export { f } from '…'`) are skipped.
- Type-only exports (`export type`, `export interface`) are skipped.

## Fixing a finding

- **Preferred: add a test in a peer file.**
  - Rust: create `<stem>_tests.rs` next to the source with
    `use super::*;` and a `#[test]` per public function. The finding's
    suggested patch creates this file for you.
  - JS/TS: create `<stem>.test.<ext>` next to the source and import
    the function from `./<stem>`. The finding's suggested patch
    creates this file for you (importing from `vitest`).
- **Also accepted: a test in the same file.**
  - Rust: a `#[cfg(test)] mod tests` at the bottom of the file with a
    `#[test]` per public function.
  - JS/TS: an in-source `describe`/`it` block (Vitest's in-source
    testing makes this ergonomic).
- **Reduce visibility.** If the function isn't actually part of the
  public API, drop the `export` keyword (JS/TS) or downgrade to
  `pub(crate)` (Rust).
- **Suppress for a specific path.** Add an `overrides:
  [builtin.tests.pub-fn-untested]` in a repo-local rule, or set
  `enabled: false` for projects where the convention doesn't apply.

## Limitations

- Peer-file detection is path-conventional, not import-aware: a test
  file at an unconventional location (e.g. `spec/foo.ts` next to
  `src/foo.ts`) will not be discovered.
- The rule does not understand re-exports — a function tested in a
  *different* file (one not on the peer-path list) still triggers a
  finding here.
- Renaming a function but not the test that calls it will mask the
  finding until the test is rewritten.
- Macros that generate tests (Rust's `parameterized!`, JS's custom
  helper wrappers) are seen as whatever text the invocation contains,
  which may or may not mention the public function by name.
- CommonJS exports (`module.exports.foo = …`) and JS `export { foo }`
  re-export lists are not detected — switch to ES-module
  `export function f` for the rule to apply.

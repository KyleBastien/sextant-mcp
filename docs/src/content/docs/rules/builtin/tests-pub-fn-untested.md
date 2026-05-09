---
title: builtin.tests.pub-fn-untested
description: Public Rust functions or exported JS/TS declarations whose name is never mentioned in an in-file test body.
sidebar:
  label: tests.pub-fn-untested
  order: 7
---

| Field | Value |
|---|---|
| **id** | `builtin.tests.pub-fn-untested` |
| **severity** | `info` |
| **category** | `tests` |
| **scope** | `file` |
| **languages** | rust, javascript, typescript, tsx |
| **evaluator** | `builtin / pub_fn_untested` |

Flags public-API definitions whose name does not appear in any test
body within the same file. Severity is `info` — this is a *signal*,
not a verdict-breaker, because public-API conventions vary across
projects and most JS/TS projects keep tests in a sibling file.

The match is intentionally heuristic and **same-file only**. Tests in
a sibling `tests/` directory, a separate integration-test crate, or a
co-located `*.test.ts` / `*.spec.ts` file are not considered.
Cross-file detection is on the roadmap once the engine grows a
repo-scope evaluator API.

## Per-language semantics

### Rust

- Only fully-public `pub fn`. `pub(crate)`, `pub(super)`, and
  `pub(in …)` are excluded — they're internal API.
- Functions inside a module annotated with `#[cfg(test)]` are
  excluded — they're test helpers, not public surface.
- "Mentioned in a test" means the function's name appears as a whole
  word inside a `#[test]`, `#[tokio::test]`, or any
  `…::test`-attributed function body. Indirect coverage through a
  helper inside `#[cfg(test)] mod` also counts — those bodies are
  pulled into the haystack too.

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
- "Mentioned in a test" means the name appears as a whole word inside
  a `describe` / `it` / `test` / `suite` / `context` / `fit` /
  `fdescribe` callback body — including chained forms like
  `it.skip(…)`, `describe.only(…)`, and `test.each(table)(name, fn)`.
  This covers Jest, Vitest, Mocha, and Jasmine in-file test patterns.

## Fixing a finding

- **Add a unit test next to the function.**
  - *Rust:* a `#[cfg(test)] mod tests` at the bottom of the file with
    a `#[test]` per public function. One focused test is enough.
  - *JS/TS:* an in-file `describe`/`it` block (Vitest's in-source
    testing makes this ergonomic) or a sibling `*.test.ts` /
    `*.spec.ts` plus a rule override.
- **Reduce visibility.** If the function isn't actually part of the
  public API, drop the `export` keyword (JS/TS) or downgrade to
  `pub(crate)` (Rust).
- **Suppress for a specific path.** Add an
  `overrides: [builtin.tests.pub-fn-untested]` in a repo-local rule,
  or set `enabled: false` for projects where the convention doesn't
  apply.

## Limitations

- The rule does not understand re-exports — a function tested in a
  *different* file still triggers a finding here. JS/TS projects that
  keep tests in `*.test.ts` siblings will see this rule fire often;
  consider disabling it or scoping it to a `src/` subtree via an
  override.
- Renaming a function but not the test that calls it will mask the
  finding until the test is rewritten.
- Macros that generate tests (Rust's `parameterized!`, JS's custom
  helper wrappers) are seen as whatever text the invocation contains,
  which may or may not mention the public function by name.
- CommonJS exports (`module.exports.foo = …`) and JS `export { foo }`
  re-export lists are not detected — switch to ES-module
  `export function f` for the rule to apply.

## See also

- [Authoring rules → override a built-in](/sextant-mcp/rules/authoring/#override-a-built-in) —
  how to disable this rule per-project.

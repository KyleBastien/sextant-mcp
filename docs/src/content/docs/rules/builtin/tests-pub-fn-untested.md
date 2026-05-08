---
title: builtin.tests.pub-fn-untested
description: Public Rust functions whose name is never mentioned in a #[test] body in the same file.
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
| **languages** | rust |
| **evaluator** | `builtin / pub_fn_untested` |

Flags `pub fn` definitions whose name does not appear in any `#[test]`
function within the same file. Severity is `info` — this is a
*signal*, not a verdict-breaker, because public API conventions vary
across projects.

The match is intentionally heuristic:

- **Rust only.**
- **Same file only.** Tests in a sibling `tests/` directory or in a
  separate integration-test crate are not considered. Cross-file
  detection is on the roadmap once the engine grows a repo-scope
  evaluator API.
- **Only fully-public `pub fn`.** `pub(crate)`, `pub(super)`, and
  `pub(in …)` are excluded — they're internal API.
- Functions inside a module annotated with `#[cfg(test)]` are
  excluded — they're test helpers, not public surface.
- "Mentioned in a test" means the function's name appears as a whole
  word inside a `#[test]`, `#[tokio::test]`, or any
  `…::test`-attributed function body. Indirect coverage through a
  helper does not count.

## Fixing a finding

- **Add a unit test next to the function.** The Rust convention is a
  `#[cfg(test)] mod tests` at the bottom of the file with a
  `#[test]` per public function. Even one focused test per function
  is enough to silence this rule.
- **Reduce the visibility.** If the function isn't actually part of
  the public API, downgrading to `pub(crate)` removes it from this
  rule's scope and tightens the encapsulation.
- **Suppress for a specific file.** Add an
  `overrides: [builtin.tests.pub-fn-untested]` in a repo-local rule,
  or set `enabled: false` for projects where the convention doesn't
  apply.

## Limitations

- The rule does not understand re-exports — a function tested in a
  *different* file still triggers a finding here.
- Renaming a function but not the test that calls it will mask the
  finding until the test is rewritten.
- Macros that generate tests (e.g., `parameterized!`) are seen as
  whatever text the macro invocation contains, which may or may not
  mention the public function by name.

## See also

- [Authoring rules → override a built-in](/sextant-mcp/rules/authoring/#override-a-built-in) —
  how to disable this rule per-project.

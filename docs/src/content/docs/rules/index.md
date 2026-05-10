---
title: Rules catalog
description: The seven built-in rules that ship with Sextant.
sidebar:
  label: Catalog
  order: 1
---

Sextant ships seven built-in rules covering size, complexity,
duplication, and tests. They're embedded in the binary and load
automatically — no `.sextant/rules/` entries required.

| Rule id | Severity | Languages | Summary |
|---|---|---|---|
| [`builtin.size.file-length`](/sextant-mcp/rules/builtin/size-file-length/) | warn | all | File exceeds line-count thresholds. |
| [`builtin.size.fn-length`](/sextant-mcp/rules/builtin/size-fn-length/) | warn | rust, python, go, java, ts/tsx/js | Function body too long. |
| [`builtin.size.param-count`](/sextant-mcp/rules/builtin/size-param-count/) | warn | rust, python, go, java, ts/tsx/js | Too many parameters on one function. |
| [`builtin.complexity.cyclomatic`](/sextant-mcp/rules/builtin/complexity-cyclomatic/) | warn | rust, python, go, java, ts/tsx/js | Too many independent control-flow paths. |
| [`builtin.complexity.nesting`](/sextant-mcp/rules/builtin/complexity-nesting/) | warn | rust, python, go, java, ts/tsx/js | Deeply nested control structures. |
| [`builtin.duplication.tokens`](/sextant-mcp/rules/builtin/duplication-tokens/) | warn | rust, python, go, java, ts/tsx/js | Repeated structurally-identical code. |
| [`builtin.tests.pub-fn-untested`](/sextant-mcp/rules/builtin/tests-pub-fn-untested/) | info | rust, ts/tsx/js | Public function or exported declaration never mentioned in a test in the same file. |

All seven are `file`-scope, so they fire in both diff and whole-file
mode (with diff-mode filtering findings to changed lines). Tune their
thresholds in
[`.sextant/config.toml`](/sextant-mcp/configuration/).

## Authoring your own

[Authoring rules](/sextant-mcp/rules/authoring/) covers the full
schema for `.sextant/rules/<name>.md` files — `regex`, `ast`, and
`llm` evaluators, frontmatter fields, validation flow.

## Installing a rule pack

For curated rule sets shipped by another team (or another repo of
yours), see [Rule packs](/sextant-mcp/packs/). Packs are
hash-locked, agent-resistant bundles installed via `sextant rules add
github:owner/repo@tag`. The first shipped pack is the
[TypeScript pack](/sextant-mcp/packs/typescript/).

## See also

- [Rule concept](/sextant-mcp/concepts/rule/) — the data model.
- [Authoring rules](/sextant-mcp/rules/authoring/) — write your own.
- [Rule packs](/sextant-mcp/packs/) — install or author a shareable
  rule bundle.
- [`sextant rules`](/sextant-mcp/cli/rules/) — list, explain,
  validate, install.

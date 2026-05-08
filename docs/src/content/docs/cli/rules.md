---
title: sextant rules
description: List, explain, and validate Sextant rules.
sidebar:
  order: 3
---

`sextant rules` is rule introspection. Three subcommands.

## `sextant rules list`

```sh
sextant rules list
```

Prints every rule loaded for the current repo — built-ins plus anything
under `.sextant/rules/`. Each line shows id, severity, scope, source,
and the one-line description from the rule's frontmatter.

```text
builtin.size.file-length          warn   file   builtin   Files that exceed the configured line-count thresholds.
builtin.size.fn-length            warn   file   builtin   Functions whose body spans more than the configured number of lines.
builtin.size.param-count          warn   file   builtin   Functions that take more than the configured number of parameters.
builtin.complexity.cyclomatic     warn   file   builtin   Functions with too many independent control-flow paths.
builtin.complexity.nesting        warn   file   builtin   Functions with too many nested control structures.
builtin.duplication.tokens        warn   file   builtin   Repeated runs of structurally-identical code within a file.
builtin.tests.pub-fn-untested     info   file   builtin   Public Rust functions whose name is never mentioned in a `#[test]` body in the same file.
project.no-todo                   warn   file   repo      Disallow TODO comments in production code.
```

Use this to confirm a new rule loaded (look for it in the `repo` source
column) or to find a rule id to pass to `explain`.

## `sextant rules explain <id>`

```sh
sextant rules explain builtin.size.fn-length
```

Prints the rule's full markdown body — the same content shown by the
MCP `explain_rule` tool. The body explains *why* the rule exists and
how to fix a finding, so it's the right thing to read when an
unfamiliar rule fires.

If the id doesn't exist, the command exits 2 and lists fuzzy matches.

## `sextant rules check <path>`

```sh
sextant rules check .sextant/rules/no-todo.md
```

Validates a single rule file's frontmatter without loading it into the
engine. Useful as a pre-commit hook on a rule-authoring workflow.

Errors are printed with the field name and the validation failure:

```text
.sextant/rules/no-todo.md:
  - severity: must be one of `info`, `warn`, `error` (got `warning`)
  - evaluator.pattern: required when evaluator.type is `regex`
```

Exit code is `0` on success, `2` on validation failure.

## See also

- [Authoring rules](/sextant-mcp/rules/authoring/) — full rule schema.
- [Rules catalog](/sextant-mcp/rules/) — built-in rules.
- [`explain_rule` MCP tool](/sextant-mcp/mcp/tools/explain-rule/) — same
  behaviour, exposed to agents.

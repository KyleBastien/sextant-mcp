---
title: Authoring rules
description: Write a custom rule in .sextant/rules/.
sidebar:
  order: 2
---

Repo-local rules live at `.sextant/rules/<name>.md`. The file is
markdown with YAML frontmatter — same shape as the built-in rules
embedded in `crates/sextant-rules/rules/builtin/`.

The body of the file becomes user-facing documentation: it's what
[`explain_rule`](/sextant-mcp/mcp/tools/explain-rule/) and
[`sextant rules explain`](/sextant-mcp/cli/rules/) print, and it's what
the LLM evaluator (if any) uses as its prompt.

## Frontmatter schema

```yaml
---
id: project.no-todo                # required, dotted, globally unique
name: "No TODO comments"           # required, human-readable
description: "One-liner."          # required, shown in `rules list`
severity: warn                     # info | warn | error
category: style                    # complexity | size | duplication |
                                   # tests | reliability | style |
                                   # security | docs | { custom: "<name>" }
scope: file                        # diff | file | repo
languages: [rust, python]          # omit = all languages
evaluator:                         # see below
  type: regex
  pattern: '\bTODO\b'
  exclude_paths: ["**/tests/**"]
enabled: true                      # default true
overrides: []                      # rule ids this rule disables
tags: [style, todo]
---
```

| Field | Required | Notes |
|---|---|---|
| `id` | yes | Dotted, globally unique. Convention: `<source>.<category>.<short-name>`. |
| `name` | yes | Human-readable. |
| `description` | yes | One-liner shown in `rules list`. |
| `severity` | yes | `info`, `warn`, or `error`. |
| `category` | yes | Built-in enum or `{ custom: "<name>" }`. |
| `scope` | yes | `diff`, `file`, or `repo`. |
| `languages` | no | Whitelist. Empty = all languages. |
| `evaluator` | yes | `regex` or `llm` for repo rules. See below. |
| `enabled` | no | Default `true`. Set `false` for a stub overriding a built-in. |
| `overrides` | no | Rule ids this rule replaces. |
| `tags` | no | Free-form labels. |

## Evaluator types

### `regex` — line-by-line pattern match

```yaml
evaluator:
  type: regex
  pattern: '\.unwrap\('
  exclude_paths: ["**/tests/**", "**/*_test.rs"]
```

| Field | Required | Notes |
|---|---|---|
| `pattern` | yes | Standard Rust regex. Matched per line. |
| `exclude_paths` | no | Glob patterns that skip files. |

Cheapest authoring option. The regex runs against each line of each
file in scope; every match is one finding pointing at that line.

### `llm` — LLM-as-judge

```yaml
evaluator:
  type: llm
  model: claude-sonnet-4-6        # optional; falls back to [judge].model
  max_tokens: 1024                # optional
  temperature: 0.0                # optional
  exclude_paths: ["**/tests/**"]
```

The rule body is the prompt. Placeholders `{{path}}`, `{{code}}`, and
`{{rule.id}}` get substituted at evaluation time. Output is constrained
via tool-use to well-typed [Findings](/sextant-mcp/concepts/finding/) —
no JSON parsing failures.

Requires `[judge]` in `.sextant/config.toml` and the corresponding API
key in env. See [Configuration → judge](/sextant-mcp/configuration/judge/).

### `builtin` — Rust evaluator

Reserved for built-in rules. Don't use this in repo-local rules.

## After authoring

1. **Validate**: `sextant rules check .sextant/rules/<name>.md` — catches
   YAML errors and missing fields without fully loading the rule.
2. **Confirm load**: `sextant rules list | grep <id>` — should show the
   rule with `source: repo`.
3. **Try it**: `sextant grade` and look for findings.
4. **Read it back**: `sextant rules explain <id>` — verify the body
   formats well.

## Style

- The body is shown verbatim by `explain_rule`. Treat it as user-
  facing documentation: explain *why* the rule exists and how to fix a
  finding.
- Calibrate severity. `error` should block real bugs; `warn` is "fix
  when convenient"; `info` is informational.
- For LLM rules, write the prompt to ask for *concrete* findings tied
  to specific lines. Vague prompts produce vague output.

## Override a built-in

Repo-local rules with the same `id` as a built-in replace it. To turn
one off entirely, ship a stub:

```yaml
---
id: builtin.size.fn-length
name: "(disabled)"
description: "x"
severity: info
category: size
enabled: false
evaluator: { type: regex, pattern: "(?!)" }
---
```

The `(?!)` regex never matches, the stub is `enabled: false`, and the
built-in is replaced by an inert rule.

## Examples

### "No `unwrap()` outside tests"

```yaml
---
id: project.no-unwrap
name: "No unwrap() in production code"
description: "Forbid .unwrap() outside test files."
severity: warn
category: reliability
scope: file
languages: [rust]
evaluator:
  type: regex
  pattern: '\.unwrap\('
  exclude_paths: ["**/tests/**", "**/*_test.rs", "**/benches/**"]
tags: [rust, panics]
---

# No unwrap() in production code

`.unwrap()` panics on `None` / `Err`. In production code that's
almost always a bug — the program crashes instead of handling the
error path.

## Fixing a finding

- Use `?` to propagate errors up.
- Use `.expect("reason")` if the panic is genuinely unreachable, with
  a comment explaining why.
- Use pattern matching to handle both arms explicitly.
```

### LLM rule: "API surface comments"

```yaml
---
id: project.api-surface-comments
name: "Public API needs a comment"
description: "Public functions / types should have a doc-comment explaining intent."
severity: info
category: docs
scope: file
languages: [rust]
evaluator:
  type: llm
  model: claude-sonnet-4-6
  exclude_paths: ["**/tests/**", "**/*_test.rs"]
---

# Public API needs a comment

You are reviewing `{{path}}` for missing documentation on the public
API surface.

Look for `pub fn`, `pub struct`, `pub enum`, and `pub trait` items
that lack a `///` or `//!` doc-comment immediately above them.
Internal items (`pub(crate)`, `pub(super)`) are out of scope.

For each violation, return a finding pointing at the line of the
public item with severity `info` and a message naming the item and
suggesting a one-line description.

```code
{{code}}
```

## See also

- [Rule concept](/sextant-mcp/concepts/rule/) — the data model.
- [Evaluator concept](/sextant-mcp/concepts/evaluator/) — `regex` vs
  `llm`.
- [Configuration → judge](/sextant-mcp/configuration/judge/) — LLM
  provider config.

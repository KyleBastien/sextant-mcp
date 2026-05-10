---
title: Rule
description: A single check, with metadata describing how and where it runs.
sidebar:
  order: 2
---

A **rule** is one check that Sextant runs against your code. It has an id,
some metadata, and an evaluator that does the actual work.

## Anatomy

```yaml
---
id: builtin.size.fn-length
name: "Function length"
description: "Functions whose body spans more than the configured number of lines."
severity: warn
category: size
scope: file
languages: [rust, python, go, java, typescript, tsx, javascript]
evaluator:
  type: builtin
  name: fn_length
enabled: true
tags: [size, complexity]
---

# Function length

Long functions tend to bundle multiple responsibilities…
```

The frontmatter is metadata. The body is human-facing documentation —
shown by `sextant rules explain` and the MCP `explain_rule` tool.

## Fields

| Field | Type | Required | Notes |
|---|---|---|---|
| `id` | dotted string | yes | Globally unique. Convention: `<source>.<category>.<short-name>`. Built-ins are namespaced under `builtin.*`; repo rules can use anything. |
| `name` | string | yes | Human-readable title. |
| `description` | string | yes | One-liner shown in `rules list` output. |
| `severity` | `info` \| `warn` \| `error` | yes | See [Finding → Severity](/sextant-mcp/concepts/finding/#severity). |
| `category` | enum | yes | `size`, `complexity`, `duplication`, `tests`, `reliability`, `style`, `security`, `docs`, or `{ custom: "<name>" }`. |
| `scope` | `diff` \| `file` \| `repo` | yes | What slice the evaluator sees. See [Scopes](/sextant-mcp/concepts/scopes/). |
| `languages` | array | no | Whitelist of language tags. Empty means all languages. |
| `evaluator` | object | yes | The check itself. See [Evaluator](/sextant-mcp/concepts/evaluator/). |
| `enabled` | bool | no | Default `true`. Set `false` to ship a disabled stub (useful to override a built-in). |
| `overrides` | array of ids | no | Rule ids this rule replaces. |
| `tags` | array | no | Free-form labels — surfaced in JSON, used for filtering. |

## Where rules come from

Rules have one of three sources, available as the `source` field on
`list_rules` output:

- **`builtin`** — embedded in the `sextant` binary at compile time. The
  seven rules in [Rules catalog](/sextant-mcp/rules/) ship today.
- **`vendor:<pack>`** — installed from a [rule pack](/sextant-mcp/packs/)
  via `sextant rules add`. Lives under `.sextant/rules/vendor/<pack>/`,
  hashes recorded in `.sextant/rules.lock`. Vendor rules are
  immutable: editing or deleting a file aborts the next grade.
- **`repo`** — markdown files in `.sextant/rules/**/*.md` (excluding
  the `vendor/` subtree) of the repo being graded.

Priority on id collision is **builtin → vendor → repo**, in
increasing order:

| Conflict | Outcome |
|---|---|
| Repo rule with same id as a builtin | Repo wins (logged). |
| Vendor rule with same id as a builtin | Vendor wins (logged). |
| Repo rule with same id as a vendor rule | **Hard load error.** Vendor rules are not shadowable. |

`overrides:` lists in a rule's frontmatter only flow downward —
vendor packs can disable built-ins, but a repo rule can't disable a
vendor rule. The point is that pack authors' intent survives in the
codebase even when an agent tries to silence it; see
[Rule packs](/sextant-mcp/packs/) for the full integrity model.

## Authoring

Rule files live in `.sextant/rules/<name>.md`. The `name` of the file
doesn't matter — the loader keys off the frontmatter `id`. Read
[Authoring rules](/sextant-mcp/rules/authoring/) for the full schema and
evaluator-specific fields. To package a set of rules for distribution,
see [Authoring a pack](/sextant-mcp/packs/authoring/).

## See also

- [Rules catalog](/sextant-mcp/rules/) — the seven built-in rules.
- [Rule packs](/sextant-mcp/packs/) — shareable, integrity-locked rule
  bundles.
- [Authoring rules](/sextant-mcp/rules/authoring/) — write your own.
- [Evaluator](/sextant-mcp/concepts/evaluator/) — `builtin`, `regex`,
  `ast`, or `llm`.

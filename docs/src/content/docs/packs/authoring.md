---
title: Authoring a pack
description: Layout, manifest, and the AST evaluator type for shareable rule bundles.
sidebar:
  order: 4
---

A pack is a directory of rule markdown files plus a `pack.toml`
manifest. To ship one, push that directory to a public GitHub repo;
your users install it with `sextant rules add github:<you>/<repo>@<tag>`.

## Layout

```
my-pack/                    # repo root, or a subdirectory of one
├── pack.toml               # required
├── README.md               # optional, but recommended — shown in your repo
└── rules/
    ├── rule-one.md
    ├── rule-two.md
    └── …
```

The `rules/` subdirectory is non-negotiable: the loader looks for
`.md` files there. Anything outside `rules/` (a README, a LICENSE)
is part of the pack's hash record but isn't loaded as a rule.

## `pack.toml`

```toml
name = "typescript"
version = "0.1.0"
description = "Strict TypeScript rules for AI agents."
homepage = "https://github.com/yourname/your-pack"
license = "MIT"
sextant = ">=0.1.0"          # min sextant version (parsed, not yet enforced)
```

| Field | Required | Notes |
|---|---|---|
| `name` | yes | Becomes the directory name under `.sextant/rules/vendor/`. Use `kebab-case` or `snake_case`; the loader will reject mismatched names. |
| `version` | recommended | SemVer-style. Surfaced in `sextant rules add` output. |
| `description` | recommended | One-liner. |
| `homepage` | optional | URL. |
| `license` | recommended | SPDX identifier. |
| `sextant` | optional | Version constraint string. Parsed today; enforced in a future release. |

## Rule files

Each rule is a markdown file with YAML frontmatter, identical to the
[repo-local rule schema](/sextant-mcp/rules/authoring/) — same fields,
same evaluator types — with one convention:

**Use `vendor.<pack-name>.<short-id>` as the rule id.**

```yaml
---
id: vendor.typescript.no-any
name: "No `any` type"
description: "Bans `any` in any type position."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: '((predefined_type) @t (#eq? @t "any"))'
  capture: t
tags: [strict, types]
---

# No `any` type

…body shown by `sextant rules explain`…
```

The `vendor.<pack>.` prefix isn't enforced syntactically, but it's how
users tell vendor rules apart in `sextant rules list` and how the
`source: vendor:<pack>` provenance pairs cleanly with the id.

## The `ast` evaluator

Most pack rules will want the
[`ast` evaluator](/sextant-mcp/concepts/evaluator/#ast--tree-sitter-query),
which runs a tree-sitter query over each file's parse tree. Compared
to `regex`, AST queries:

- Don't fire on matches inside string literals or comments (those are
  different node kinds).
- Can target type-position vs. value-position keywords precisely.
- Support `not_under: [<node-kind>]` for context-sensitive exemptions
  ("allow `unknown` only inside a `catch_clause`").

```yaml
evaluator:
  type: ast
  query: '((predefined_type) @t (#eq? @t "unknown"))'
  capture: t
  not_under: [catch_clause]
  message: "no `unknown` here — use a generic"
```

| Field | Required | Notes |
|---|---|---|
| `query` | yes | Tree-sitter query S-expression. Compiled once per language listed in `languages`. |
| `capture` | no | Capture name to anchor the finding line. Defaults to the first capture in the query. |
| `message` | no | Override message. Falls back to `<rule.name>: matched <snippet>`. |
| `not_under` | no | Drop a match if any ancestor's node kind is in this list. |

Authoring tip: keep one `pack.toml` checked into the repo while you
iterate, install via `file:./path/to/pack`, and `sextant rules update`
re-syncs each time you change a rule. When the pack is ready, tag a
release and your users switch their install spec to `github:`.

## Severity and verdict

Pack rules typically ship at `severity: error` so they hard-block
under `[verdict] max_errors = 0`. That's the strictest signal you
can send: violations of *your* pack's rules are not warnings.

If a rule is genuinely advisory, mark it `severity: info` (or `warn`)
and include that in the pack's README so users tune their thresholds
appropriately.

## Versioning

Tags should be SemVer:

- **PATCH** — query refinement that catches new cases without
  breaking existing ones.
- **MINOR** — new rule, or expanded rule scope (more files match).
- **MAJOR** — rule removal, id rename, or behavior that flips
  previously-passing code to failing.

Document breaking changes in your release notes. Users update by
re-running `sextant rules add <spec>@<new-tag>`.

## Distribution

The recommended layout for a dedicated pack repo:

```
your-pack-repo/
├── pack.toml
├── README.md
├── LICENSE
├── rules/*.md
└── tests/                          # optional but recommended
    └── …                           # see Sextant's own packs/typescript tests
                                    # for a fixture pattern
```

If your repo ships multiple packs, put each one under its own subdir
and have users install with the `#<subdir>` form:

```sh
sextant rules add github:you/repo@v1.0.0#packs/typescript
sextant rules add github:you/repo@v1.0.0#packs/python
```

## See also

- [Rule packs overview](/sextant-mcp/packs/)
- [Installing packs](/sextant-mcp/packs/installing/)
- [TypeScript pack](/sextant-mcp/packs/typescript/) — a reference
  implementation; the source is at
  [`packs/typescript/`](https://github.com/kylebastien/sextant-mcp/tree/main/packs/typescript).
- [Authoring rules](/sextant-mcp/rules/authoring/) — the underlying
  rule schema, including the `ast` evaluator.

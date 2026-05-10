---
title: Rule packs
description: Shareable, integrity-locked bundles of Sextant rules.
sidebar:
  label: Overview
  order: 1
---

A **rule pack** is a curated bundle of Sextant rules that another team
or project ships, fetched from GitHub or a local path. Install one with
`sextant rules add` and every grade in your repo picks up the new
rules.

```sh
sextant rules add github:kylebastien/sextant-mcp@v0.2.0#packs/typescript
```

Packs are designed for **agent codebases**: the rules that ship in a
pack cannot be turned off by the agent, by an `enabled: false` edit,
or by an `overrides:` list in another rule. Tampering with a pack
file fails the next grade with a hash-mismatch error. The intent is
durable: once your team's house style is in a pack, it stays enforced.

## Install, update, remove

Three CLI verbs:

```sh
# Install a pack from GitHub at a tag (or branch).
sextant rules add github:owner/repo@v1.0.0[#subdir]

# Install from a local path (handy for dev / CI fixtures).
sextant rules add file:./path/to/pack

# Re-fetch every installed pack at its pinned ref (no-op if hashes match).
sextant rules update

# Re-fetch a specific pack.
sextant rules update typescript

# Drop a pack and its lock entry.
sextant rules remove typescript
```

Full reference: [Installing packs](/sextant-mcp/packs/installing/).

## What gets written

Three things land in your repo when you `add` a pack:

```
.sextant/
├── rules.lock                          # pinned source + per-file SHA-256
└── rules/
    └── vendor/
        └── typescript/                 # one directory per installed pack
            ├── pack.toml               # pack manifest (name, version, …)
            ├── README.md
            └── rules/
                ├── no-any.md
                ├── no-as-cast.md
                └── …                   # one markdown file per rule
```

Both `.sextant/rules.lock` and `.sextant/rules/vendor/` are meant to be
committed. The lock file is the source of truth for which packs are
installed; the `vendor/` tree is the actual rule content the engine
loads. See [Lock file](/sextant-mcp/packs/installing/#the-lock-file)
for the integrity model.

## Why agents can't disable a pack rule

Repo-local rules at `.sextant/rules/<name>.md` always cede priority to
vendor pack rules with the same id. Concretely:

- `enabled: false` inside a vendor rule file has no effect — the lock-
  integrity check would catch the edit before the rule loads.
- A repo rule with `overrides: [vendor.typescript.no-any]` is silently
  ignored. Override scoping only flows downward (vendor can disable a
  built-in; repo cannot disable a vendor).
- A repo rule with the same `id` as a vendor rule is a hard load error
  ("repo rule … shadows vendor pack rule of the same id").
- Editing a vendor file changes its SHA-256; the next grade fails.
- Deleting a vendor file is also caught (the lock still references it).

The whole point is that the pack author's intent survives in the
codebase even when an agent tries to silence it.

## What's available

| Pack | Source | Summary |
|---|---|---|
| [`typescript`](/sextant-mcp/packs/typescript/) | `kylebastien/sextant-mcp@vX#packs/typescript` | Strict TypeScript: ban `any`, `unknown`, casts, `{}`, branded types, and the usual footguns. |

More packs are planned. To author your own, see
[Authoring a pack](/sextant-mcp/packs/authoring/).

## See also

- [Installing packs](/sextant-mcp/packs/installing/) — `sextant rules
  add/update/remove`, the lock file, and bypass attempts.
- [TypeScript pack](/sextant-mcp/packs/typescript/) — the first
  shipped pack, rule-by-rule.
- [Authoring a pack](/sextant-mcp/packs/authoring/) — `pack.toml`,
  layout, and the `ast` evaluator.
- [Rule concept](/sextant-mcp/concepts/rule/#where-rules-come-from) —
  how vendor packs slot into the loader.

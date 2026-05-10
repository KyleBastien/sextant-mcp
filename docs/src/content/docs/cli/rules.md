---
title: sextant rules
description: List, explain, validate, and manage vendor rule packs.
sidebar:
  order: 3
---

`sextant rules` is the umbrella for everything to do with rules — the
seven built-ins, your repo-local files, and any vendor pack you've
installed. Six subcommands.

## `sextant rules list`

```sh
sextant rules list
```

Prints every rule loaded for the current repo — built-ins plus anything
under `.sextant/rules/`. Each line shows id, severity, scope, source,
and the one-line description from the rule's frontmatter.

```text
builtin.size.file-length              warn    file   builtin             Files that exceed the configured line-count thresholds.
builtin.size.fn-length                warn    file   builtin             Functions whose body spans more than the configured number of lines.
builtin.complexity.cyclomatic         warn    file   builtin             Functions with too many independent control-flow paths.
…
project.no-todo                       warn    file   repo                Disallow TODO comments in production code.
vendor.typescript.no-any              error   file   vendor:typescript   Bans the `any` type in any type position.
vendor.typescript.no-as-cast          error   file   vendor:typescript   Bans the `as` cast (`x as Foo`).
…
```

The `source` column distinguishes built-ins, repo-local rules, and
vendor pack rules. Vendor rules render as `vendor:<pack-name>` so the
provenance is unambiguous when several packs are installed.

Use this command to confirm a new rule loaded, or to find a rule id
to pass to `explain`.

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

## `sextant rules add <spec> [--name <override>]`

Install a vendor rule pack from GitHub or a local path. `<spec>` takes
one of three forms:

```sh
# A pack at the root of a GitHub repo, pinned to a tag.
sextant rules add github:owner/repo@v1.0.0

# A pack under a subdirectory of a repo (one repo, many packs).
sextant rules add github:owner/repo@v1.0.0#packs/typescript

# A local path — handy for dev / fixtures.
sextant rules add file:./packs/typescript
```

The pack is cloned into a tempdir, validated (must contain `pack.toml`),
SHA-256-hashed, then atomically moved into
`.sextant/rules/vendor/<pack-name>/`. The lock entry (or the whole
`.sextant/rules.lock`, if it didn't exist) is written last.

`--name <override>` overrides the pack name from `pack.toml` —
mostly useful for `file:`-sourced dev work where the manifest doesn't
yet match the directory you want.

Full reference: [Installing packs](/sextant-mcp/packs/installing/).

## `sextant rules update [<pack> ...]`

Re-fetch each installed pack at its **pinned** ref (the `ref` field
of the lock entry). With no arguments, refreshes every pack.

```sh
sextant rules update                  # all installed packs
sextant rules update typescript       # one pack
```

This is for sync, not version-bumping. To bump to a new tag, re-run
`sextant rules add github:<repo>@<new-tag>` against the same pack.

The command is idempotent — if every hash already matches what's in
the lock, the output is "pack `<name>` already up to date" and
nothing changes on disk.

## `sextant rules remove <pack>`

```sh
sextant rules remove typescript
```

Deletes the pack directory and drops its entry from the lock. The
next grade no longer sees those rules.

## See also

- [Authoring rules](/sextant-mcp/rules/authoring/) — full rule schema.
- [Rules catalog](/sextant-mcp/rules/) — built-in rules.
- [Rule packs](/sextant-mcp/packs/) — the vendor-pack model end-to-end.
- [`explain_rule` MCP tool](/sextant-mcp/mcp/tools/explain-rule/) — same
  behaviour, exposed to agents.

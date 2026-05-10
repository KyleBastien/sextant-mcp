---
title: Installing packs
description: sextant rules add / update / remove and the .sextant/rules.lock file.
sidebar:
  order: 2
---

A vendor pack arrives in three pieces: a fetch (clone the source), a
copy (stage the contents under `.sextant/rules/vendor/<name>/`), and a
hash record (`.sextant/rules.lock`). Every grade re-verifies the
hashes before loading any pack rule.

## `sextant rules add`

```sh
sextant rules add <spec> [--name <override>]
```

`<spec>` is one of three forms:

| Form | What it means |
|---|---|
| `github:owner/repo@<ref>` | Clone the repo at `<ref>` (a tag, branch, or commit SHA). The pack manifest must live at the repo root. |
| `github:owner/repo@<ref>#<subdir>` | Same, but the pack lives under `<subdir>`. Useful when one repo ships multiple packs. |
| `file:<path>[#<subdir>]` | Copy from a local path. Dev / test only — won't produce a portable lock entry until you switch to `github:`. |

The pack name comes from `pack.toml`'s `name` field. `--name` overrides
that, mostly useful when iterating on a `file:` pack that doesn't yet
have a manifest.

`add` is atomic: a partial fetch never leaves a half-installed pack
behind. The staged tempdir is hashed and validated before being moved
into place.

### What `add` writes

```
.sextant/
├── rules.lock                          # NEW or updated
└── rules/
    └── vendor/
        └── <pack-name>/                # NEW
            ├── pack.toml
            ├── README.md
            └── rules/*.md
```

Lock-file entry, generated:

```toml
[[packs]]
name = "typescript"
source = "github:kylebastien/sextant-mcp"
ref = "v0.2.0"
revision = "abc123def4567890..."        # full commit SHA
subdir = "packs/typescript"             # omitted when empty
fetched_at = "2026-05-10T12:00:00Z"

[packs.files]
"pack.toml"            = "sha256:0123…"
"README.md"            = "sha256:…"
"rules/no-any.md"      = "sha256:…"
"rules/no-as-cast.md"  = "sha256:…"
# … one entry per file in the pack
```

Commit both `.sextant/rules.lock` and `.sextant/rules/vendor/<name>/`.

## `sextant rules update`

```sh
sextant rules update [<pack> ...]
```

Re-fetches each installed pack at its pinned ref. With no arguments,
updates every pack. With pack names, updates only those.

`update` is idempotent: if every file's hash already matches what's
in the lock, the command prints "already up to date" and does
nothing. Useful when:

- A tag was force-pushed and you want to verify your local copy still
  matches.
- You're iterating on a `file:`-sourced pack during dev — `update`
  re-syncs your repo to the latest pack source.

To **bump** to a new ref, run `sextant rules add <same-spec>@<new-ref>`
again. There's no separate `--ref` flag; re-adding is the bump.

## `sextant rules remove`

```sh
sextant rules remove <pack>
```

Deletes `.sextant/rules/vendor/<pack>/` from disk and removes the
entry from `.sextant/rules.lock`. The next grade no longer sees those
rules.

## The lock file

`.sextant/rules.lock` is the single source of truth for which packs
are installed. The loader checks **before** every grade:

1. Each pack listed in the lock has a directory at
   `.sextant/rules/vendor/<name>/`.
2. Every file listed in `packs.files` exists on disk and its SHA-256
   matches.
3. Every file on disk under that pack's directory is also listed in
   `packs.files` — no untracked extras.

Any failure aborts the grade with a precise error message. This is
deliberately strict so a tampered or partially-removed pack never
silently degrades to "rule didn't fire."

### Failure modes you'll see

```
error: vendor pack `typescript`: file `rules/no-any.md` has been
       modified (hash mismatch) — restore the original contents or
       re-run `sextant rules update`

error: vendor pack `typescript`: file `rules/no-any.md` is missing
       from `.sextant/rules/vendor/typescript/` but is recorded in
       rules.lock — run `sextant rules update` or
       `sextant rules remove`

error: vendor pack `typescript`: file `rules/sneaky.md` exists on
       disk but is not in rules.lock — the pack has been tampered
       with; re-run `sextant rules add` or `sextant rules update`

error: vendor pack `typescript` directory is missing from
       .sextant/rules/vendor/

error: rule `vendor.typescript.no-any` in repo-local
       `.sextant/rules/` shadows vendor pack `typescript` rule of
       the same id; vendor pack rules are immutable — rename your
       repo rule
```

### What does *not* break the lock

- Renaming the pack directory under `.sextant/rules/vendor/`. The
  directory name must match the pack's `name` field; deviating
  errors with "pack name doesn't match directory."
- Reformatting or re-sorting `rules.lock`. The `[packs.files]` table
  is sorted alphabetically on write, so re-running `update` produces
  byte-identical output when nothing has changed — diffs stay clean.

## Bypass attempts that don't work

The pack model is designed so an agent (or a careless edit) can't
silently disable a pack rule. Specifically:

| Attempt | Outcome |
|---|---|
| Edit a vendor rule file (`enabled: false`, weaken the regex, change the message) | Hash mismatch, grade fails. |
| Delete a vendor rule file | Missing-file error. |
| Add a repo rule with `overrides: [vendor.typescript.no-any]` | Override silently ignored — repo rules can't disable vendor rules. |
| Add a repo rule with `id: vendor.typescript.no-any` | Hard load error: "shadows vendor pack rule." |
| Lower the verdict thresholds in `config.toml` | Allowed (your repo's choice), but note that pack rules ship at `severity: error`. |
| `git checkout` an older `rules.lock` and a newer `vendor/` | Hashes don't match — fails. |

The integrity check is fail-closed: anything weird about the pack
state is an error, not a warning.

## Working with `git`

`.sextant/rules/vendor/` is real code that participates in your
diffs. PR diffs that touch a pack will show:

- `rules.lock` — entry change for the affected pack
- `rules/vendor/<pack>/**` — the actual files

Both should land in the same commit so reviewers can see what shifted.

## See also

- [Rule packs overview](/sextant-mcp/packs/) — what packs are and why
  they exist.
- [`sextant rules`](/sextant-mcp/cli/rules/) — CLI reference.
- [Authoring a pack](/sextant-mcp/packs/authoring/) — `pack.toml` and
  layout.

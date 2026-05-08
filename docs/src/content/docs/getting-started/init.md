---
title: sextant init
description: Bootstrap a .sextant/ directory in a repo.
sidebar:
  order: 3
---

`sextant init` writes the starting files Sextant needs to grade a repo:
a config, a rules directory, and (depending on the template) a sample
rule. It is **idempotent** — files that already exist are left alone
unless you pass `--force`.

## Usage

```sh
sextant init
sextant init --template strict
sextant init --template rust --force
```

## What gets written

```
.sextant/
├── config.toml       # verdict thresholds and rule limits
└── rules/            # repo-local rule markdown files
    └── (varies by template)
```

`config.toml` covers the four built-in tuning sections — `[verdict]`,
`[size]`, `[complexity]`, `[duplication]` — plus a commented-out
`[judge]` block for LLM-evaluated rules.

The `.sextant/cache/` directory is created on first grade and is
git-ignored.

## Templates

| Template | What it does |
|---|---|
| `default` | Language-agnostic config and an empty `rules/` directory. |
| `rust`    | Adds a sample regex rule disallowing `.unwrap()` outside test files. |
| `python`  | Adds a sample regex rule disallowing bare `except:`. |
| `go`      | Adds a sample regex rule disallowing `panic(` outside test files. |
| `java`    | Adds a sample regex rule discouraging `printStackTrace()`. |
| `ts`      | Adds a sample regex rule disallowing `// @ts-ignore`. |
| `strict`  | Tighter thresholds (`fn_length_warn = 30`, `cyclomatic_warn = 6`) on top of `default`. |

The sample rules are intentionally trivial — copy-paste targets, not
recommendations. Read [Authoring rules](/sextant-mcp/rules/authoring/)
for the full schema.

## Flags

- `--template <name>` — choose a scaffold. Defaults to `default`.
- `--force` — overwrite existing files instead of skipping them.

## After init

1. Commit `.sextant/` to your repo. The Action and the plugin both pick
   it up by reading the working tree.
2. Run `sextant grade` once to confirm rules load.
3. (Optional) Lower the thresholds in `config.toml` once the baseline is
   green — start lenient, tighten as you go.
4. Read [Configuration](/sextant-mcp/configuration/) for the full schema.

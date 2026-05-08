---
title: sextant init
description: Bootstrap a .sextant/ directory with a config and sample rules.
sidebar:
  order: 4
---

`sextant init` writes a starter `.sextant/` directory. See the
[full guide](/sextant-mcp/getting-started/init/) for templates and
post-init steps; this page is the flag reference.

## Usage

```text
sextant init [OPTIONS]

Options:
      --template <TEMPLATE>
          Which scaffold to drop. Defaults to `default`.
          [possible values: default, rust, python, go, java, ts, strict]
      --force
          Overwrite existing files instead of skipping them.
  -h, --help
          Print help.
```

## What gets written

```
.sextant/
├── config.toml       # always written if missing
└── rules/            # always created
    └── (template-specific sample rule, if any)
```

`config.toml` is the only file every template writes. The `rules/`
directory is created empty for `default` and `strict`; language
templates drop one sample rule there.

## Idempotency

By default, init **skips** files that already exist. Re-running the
command after editing `config.toml` won't lose your changes. Pass
`--force` to overwrite.

```sh
sextant init                     # bootstrap, skipping existing files
sextant init --template rust     # add the Rust sample rule
sextant init --force             # overwrite everything
```

## Templates

Full template list and what each writes is in
[Getting started → sextant init](/sextant-mcp/getting-started/init/#templates).

## Exit codes

| Code | When |
|---|---|
| `0` | Files written or skipped successfully. |
| `2` | Filesystem error, invalid template name, or other CLI failure. |

## See also

- [Getting started → sextant init](/sextant-mcp/getting-started/init/) —
  walk-through.
- [Configuration](/sextant-mcp/configuration/) — what `config.toml`
  controls.
- [Authoring rules](/sextant-mcp/rules/authoring/) — write your own
  after init.

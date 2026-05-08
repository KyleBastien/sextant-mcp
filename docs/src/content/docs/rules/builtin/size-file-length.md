---
title: builtin.size.file-length
description: Files that exceed configured line-count thresholds.
sidebar:
  label: size.file-length
  order: 1
---

| Field | Value |
|---|---|
| **id** | `builtin.size.file-length` |
| **severity** | `warn` |
| **category** | `size` |
| **scope** | `file` |
| **languages** | all |
| **evaluator** | `builtin / file_length` |

Long files almost always do too many things. They're harder to read,
harder to test in isolation, and harder to navigate. When a single
module starts exceeding a few hundred lines it's usually time to
split it: one responsibility per file, one entry point per concept.

## Thresholds

Configure under `[size]` in `.sextant/config.toml`:

```toml
[size]
file_length_warn = 400
file_length_error = 800
```

| Setting | Default | Effect |
|---|---|---|
| `file_length_warn` | `400` | Files at or above this trigger a `warn` finding. |
| `file_length_error` | `800` | Files at or above this escalate to `error`. |

Counts every line in the file, including blanks and comments — no
clever counting heuristics. The intent is "the file is too big",
which is what the raw line count measures.

## Fixing a finding

- Identify cohesive groups of items (one type and its impls, one
  feature, one pipeline stage) and move each into its own module.
- Re-export from `mod.rs` / `lib.rs` to avoid breaking call sites.
- Move tests next to the code they cover so the split stays balanced.

## See also

- [Configuration → size](/sextant-mcp/configuration/size/) — full
  `[size]` reference.
- [`builtin.size.fn-length`](/sextant-mcp/rules/builtin/size-fn-length/) —
  related rule on functions.

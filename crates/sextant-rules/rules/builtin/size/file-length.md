---
id: builtin.size.file-length
name: "File length"
description: "Files that exceed the configured line-count thresholds."
severity: warn
category: size
scope: file
languages: []
evaluator:
  type: builtin
  name: file_length
enabled: true
tags: [size]
---

# File length

Long files almost always do too many things. They're harder to read, harder
to test in isolation, and harder to navigate. When a single module starts
exceeding a few hundred lines it's usually time to split it: one
responsibility per file, one entry point per concept.

## Thresholds

Configure under `[size]` in `.sextant/config.toml`:

```toml
[size]
file_length_warn = 400
file_length_error = 800
```

## Fixing a finding

- Identify cohesive groups of items (one type and its impls; one
  feature; one pipeline stage) and move each into its own module.
- Re-export from `mod.rs`/`lib.rs` to avoid breaking call sites.
- Move tests next to the code they cover so the split stays balanced.

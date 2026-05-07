---
id: builtin.duplication.tokens
name: "Token duplication"
description: "Repeated runs of structurally-identical code within a file."
severity: warn
category: duplication
scope: file
languages: [rust, python]
evaluator:
  type: builtin
  name: tokens_dup
enabled: true
tags: [duplication]
---

# Token duplication

Detects regions of code that share the same token structure — the same
sequence of statement and expression shapes, even with different
identifiers and literals (a "type-2" clone). Two occurrences of more
than `min_tokens` consecutive matching tokens within a single file
trigger a finding.

Each clone produces two findings, one anchored at each occurrence,
each pointing at the other. That keeps `grade_diff` honest: when only
one side of a clone is in the diff, the side actually being changed
gets flagged.

## Thresholds

Configure under `[duplication]` in `.sextant/config.toml`:

```toml
[duplication]
min_tokens = 100   # roughly ~20 lines of typical code
```

Lowering this catches more duplication at the cost of noise; raising
it flags only substantial copy-paste.

## Fixing a finding

- **Extract a helper function** — the clearest path. Two regions with
  the same shape almost always belong behind one name.
- **Parameterize over the differences** — if the clones differ only in
  the values they operate on, pass those values in.
- **Build a small data structure** — repeated `if/else` chains over an
  enum often collapse to a method on the enum or a lookup table.
- **Generalize over the type** — repeated logic across types is what
  generics are for.

## Limitations (v1)

- Within-file only. Cross-file duplication detection is on the roadmap.
- Token-kind hashing means the rule sees `let x = 1` and `let y = "s"`
  as identical structure (same kinds: `let`, `identifier`, `=`,
  literal, `;`). That's the right call for catching refactor
  candidates but means trivial structural similarity can fire — tune
  `min_tokens` higher if it's noisy in your codebase.

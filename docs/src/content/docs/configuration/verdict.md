---
title: "[verdict]"
description: When findings flip the verdict to request_changes.
sidebar:
  order: 2
---

`[verdict]` controls what counts as a blocking grade. The thresholds
are compared against the **counts** in the report — see
[Verdict](/sextant-mcp/concepts/verdict/) for how this interacts with
absolute vs regression mode.

## Schema

```toml
[verdict]
max_errors = 0
max_warns = 50
```

| Field | Type | Default | Effect |
|---|---|---|---|
| `max_errors` | `u32` | `0` | More than this many error-severity findings flips the verdict. |
| `max_warns` | `u32` | `u32::MAX` | More than this many warn-severity findings flips the verdict. |

`info` findings never affect the verdict.

## Defaults

Out of the box, Sextant blocks only on errors:

```toml
[verdict]
max_errors = 0
# max_warns omitted = effectively unlimited
```

That's because the seven built-in rules are `warn`-severity. With the
default `max_warns`, they surface as feedback without gating CI.

## Tighter

To make warns blocking too:

```toml
[verdict]
max_errors = 0
max_warns = 0   # any warn blocks
```

Combined with regression-mode grading on PRs, this means: "no PR may
introduce a new warn or error". Fine for tightly-maintained codebases;
noisy on early-stage repos.

## Looser

For a transitional period after introducing a new rule, you might
allow some headroom:

```toml
[verdict]
max_errors = 5
max_warns = 200
```

Then ratchet down as the codebase improves.

## Mode interaction

The same thresholds apply in both
[absolute and regression](/sextant-mcp/concepts/verdict/#modes) mode,
but they're compared against different count sources:

- **Absolute mode** (`sextant grade`): compared against the report's
  total counts.
- **Regression mode** (`sextant grade --pr`): compared against the
  delta's *new* counts. Pre-existing findings don't gate.

This is why the GitHub Action uses regression mode — a new error in
a PR blocks even if the repo already has 50 of them, but exposing an
existing finding doesn't.

## See also

- [Verdict](/sextant-mcp/concepts/verdict/) — the data model.
- [Finding → severity](/sextant-mcp/concepts/finding/#severity) — what
  counts as what.
- [Action → fail-on](/sextant-mcp/action/inputs/#fail-on) — how
  thresholds combine with the Action's gate.

---
title: Editor integration
description: The sextant-lsp server — live diagnostics and hover-to-explain inside any LSP-capable editor.
sidebar:
  label: Overview
  order: 1
---

`sextant-lsp` is a [Language Server Protocol][lsp] server that surfaces
Sextant findings inside any LSP-capable editor. Squiggles appear as you
type (debounced 400ms) and hovering a squiggle pops up the full rule
documentation — the same markdown `sextant rules explain` prints, without
the context switch.

## What's graded

| When | Mode | Scope |
|---|---|---|
| As you type | `did_change` (debounced) | The current buffer only — overlay text, no on-disk read. |
| On save | `did_save` | The current buffer; cross-file rules get a workspace pass. |

LLM-judged rules are **off by default** in the editor for speed and to
avoid surprise API spend. Toggle `sextant.disableLlm` to `false` if you
want them in the inner loop too.

Cross-file rules (e.g. token-duplication across files, untested public
functions in sibling crates) only fire on save through the workspace
pass — single-file mode favors latency over coverage.

## Editors

- [VS Code](/sextant-mcp/editor/vscode/) — first-class extension on the
  Marketplace.
- [Other editors](/sextant-mcp/editor/other-editors/) — Neovim, Helix,
  and any other LSP client. Point them at `sextant-lsp`.

## How it works

1. Editor opens a file; the LSP client sends `initialize` with the
   workspace folder.
2. `sextant-lsp` resolves the repo root by looking for `.sextant/` or
   `.git/`, walking up from the document if no workspace folder was
   given.
3. Each `did_change` schedules a debounced grade. After 400ms with no
   further keystrokes, the engine grades the in-memory buffer (the file
   on disk may be stale) and publishes diagnostics.
4. Each `did_save` triggers an immediate grade.
5. `textDocument/hover` finds findings whose range contains the cursor
   and renders the rule's full documentation.

## Configuration

The same `.sextant/config.toml` and `.sextant/rules/**/*.md` you'd use
for the CLI are picked up automatically. No editor-specific config.

## See also

- [Installation](/sextant-mcp/getting-started/installation/) — including
  the `sextant-lsp` binary.
- [CLI](/sextant-mcp/cli/) — the same engine for terminal use.
- [MCP server](/sextant-mcp/mcp/) — same engine for AI agents.

[lsp]: https://microsoft.github.io/language-server-protocol/

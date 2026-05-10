---
title: VS Code
description: Install Sextant for VS Code and configure live diagnostics.
sidebar:
  label: VS Code
  order: 2
---

The Sextant VS Code extension runs `sextant-lsp` against the file you're
editing and renders findings inline.

## Install

You need both the LSP binary and the extension.

### 1. Install `sextant-lsp`

```sh
cargo install --path crates/sextant-lsp --locked
```

(Or build from source: `cargo build --release --bin sextant-lsp` and
copy `target/release/sextant-lsp` somewhere on your `PATH`.)

### 2. Install the extension

Marketplace:

```sh
code --install-extension kylebastien.sextant
```

Or sideload a `.vsix` you've built locally:

```sh
cd editors/vscode
npm install
npm run compile
npx vsce package
code --install-extension sextant-0.1.0.vsix
```

## Settings

| Setting | Default | What it does |
|---|---|---|
| `sextant.serverPath` | `null` | Override the LSP binary path (absolute). When null, the extension looks up `sextant-lsp` on `PATH`. |
| `sextant.disableLlm` | `true` | Skip LLM-judged rules. Off by default to keep grades fast and avoid surprise API spend; toggle to `false` to grade LLM rules in-editor. |
| `sextant.trace.server` | `"off"` | Trace LSP traffic (`messages`, `verbose`). Output appears in the *Sextant* output channel. |

Settings are read at activation and on `workspace/didChangeConfiguration`,
so toggling `sextant.disableLlm` re-grades open documents without a
reload.

## Smoke test

1. Open a repo with a `.sextant/config.toml`.
2. Open a file you know has at least one Sextant finding (or temporarily
   add a long file to trip `builtin.size.file-length`).
3. A yellow or red squiggle should appear within ~500ms.
4. Hover the squiggle: a popover renders the rule title, severity, your
   finding's message, and the full rule body.
5. Edit the file to remove the violation; the squiggle disappears.

## Troubleshooting

### "could not find the sextant-lsp binary"

The extension can't find `sextant-lsp` on `PATH`. Either:

- Install it: `cargo install --path crates/sextant-lsp`.
- Or set `sextant.serverPath` to the absolute path of the binary.

### No squiggles appear

- Open the *Sextant* output channel and set `sextant.trace.server` to
  `verbose` to see the LSP traffic.
- Check that the file's language is in the activation list (Rust,
  Python, Go, Java, TypeScript, TSX, JavaScript, JSX).
- Confirm a `.sextant/` or `.git/` directory exists somewhere up the
  tree from the file — the LSP needs that to find the config.

### Hover popover is empty

Hover only fires on lines with findings. If there's no squiggle on the
line, there's no hover content to show.

## See also

- [Editor overview](/sextant-mcp/editor/) — what the LSP does and
  doesn't grade.
- [Configuration](/sextant-mcp/configuration/) — what the LSP reads from
  `.sextant/`.
- [CLI](/sextant-mcp/cli/) — same engine for terminal use.

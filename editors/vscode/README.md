# Sextant for VS Code

Live code-quality grading inside the editor, powered by [Sextant][sextant].
Squiggles appear as you type; hover any squiggle to read the rule's full
documentation without leaving the file.

## Install

The extension talks to a `sextant-lsp` binary. Install both:

```sh
# 1. Install the LSP binary on your PATH
cargo install --git https://github.com/kylebastien/sextant-mcp sextant-lsp

# 2. Install the extension
code --install-extension kylebastien.sextant-mcp
```

Or build from source: clone the repo, run `cargo install --path
crates/sextant-lsp`, then `cd editors/vscode && npm install && npm run
compile && npx vsce package` and sideload the produced `.vsix`.

## Settings

| Setting | Default | What it does |
|---|---|---|
| `sextant.serverPath` | `null` | Override the LSP binary path. |
| `sextant.disableLlm` | `true` | Skip LLM rules (fast, no API spend). Set to `false` to grade LLM rules in-editor. |
| `sextant.trace.server` | `"off"` | Trace LSP traffic (`messages`, `verbose`). |

## What's graded

Single-file grades fire on every keystroke (debounced 400ms) and on save.
Cross-file rules (clones across files, etc.) only fire on save through the
underlying `sextant grade` workspace pass. LLM-judged rules are skipped by
default — toggle `sextant.disableLlm` off to opt in.

[sextant]: https://kylebastien.github.io/sextant-mcp/

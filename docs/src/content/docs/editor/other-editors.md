---
title: Neovim, Helix, and other editors
description: Wiring sextant-lsp into LSP clients other than VS Code.
sidebar:
  label: Other editors
  order: 3
---

Any editor that speaks LSP can host `sextant-lsp`. We don't ship configs
for them yet, but the integration is a few lines.

## Neovim (`nvim-lspconfig`)

```lua
-- After installing sextant-lsp on PATH (`cargo install --path crates/sextant-lsp`):
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")

if not configs.sextant then
  configs.sextant = {
    default_config = {
      cmd = { "sextant-lsp", "--stdio" },
      filetypes = {
        "rust", "python", "go", "java",
        "typescript", "typescriptreact",
        "javascript", "javascriptreact",
      },
      root_dir = lspconfig.util.root_pattern(".sextant", ".git"),
      settings = {},
      init_options = { disableLlm = true },
    },
  }
end

lspconfig.sextant.setup({})
```

## Helix (`languages.toml`)

```toml
[language-server.sextant]
command = "sextant-lsp"
args = ["--stdio"]
config = { disableLlm = true }

[[language]]
name = "rust"
language-servers = ["rust-analyzer", "sextant"]

# Repeat the language-servers extension for any other language you use.
```

## Sublime Text (`LSP` package)

```json
{
  "clients": {
    "sextant": {
      "enabled": true,
      "command": ["sextant-lsp", "--stdio"],
      "selector": "source.rust | source.python | source.go | source.java | source.ts | source.tsx | source.js | source.jsx",
      "initializationOptions": { "disableLlm": true }
    }
  }
}
```

## Notes that apply everywhere

- **Repo root resolution.** The LSP walks up from the document looking
  for `.sextant/` then `.git/`. If your editor sets a workspace folder,
  that takes precedence.
- **`disableLlm`.** On by default in [our VS Code
  extension](/sextant-mcp/editor/vscode/) and recommended elsewhere — LLM
  rules can be slow and may incur API spend. Toggle off only when you
  want them in the inner loop.
- **Hover.** All clients should render the markdown body returned by
  `textDocument/hover`. If your client strips formatting, the rule
  documentation will still be readable.
- **Diagnostics.** Square `code` field on the diagnostic carries the
  rule id (e.g. `builtin.size.file-length`); use it for client-side
  filtering or quickfix integrations.

## See also

- [Editor overview](/sextant-mcp/editor/)
- [VS Code](/sextant-mcp/editor/vscode/)

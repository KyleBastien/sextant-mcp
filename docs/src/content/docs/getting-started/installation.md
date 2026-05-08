---
title: Installation
description: Install the sextant CLI and sextant-mcp server.
sidebar:
  order: 1
---

Sextant ships two binaries:

- **`sextant`** — the CLI used for ad-hoc grading and by the GitHub Action.
- **`sextant-mcp`** — the MCP server used by Claude Code, Claude Desktop,
  and any other MCP client.

Most users want both on `PATH`. The CLI alone is enough for CI; the MCP
server alone is enough for editor-only use.

## Release builds (recommended)

Pre-built archives for Linux (x86_64, aarch64), macOS (x86_64, aarch64),
and Windows (x86_64) are attached to every GitHub release.

1. Pick the archive matching your platform from the
   [Releases page](https://github.com/kylebastien/sextant-mcp/releases).
2. Verify the SHA-256 against `SHA256SUMS` in the same release.
3. Extract and place `sextant` and `sextant-mcp` somewhere on `PATH`
   (e.g., `~/.local/bin`, `/usr/local/bin`).

```sh
SEXTANT_VERSION=v0.1.0
ARCH=x86_64-unknown-linux-musl
curl -L -o sextant.tar.gz \
  "https://github.com/kylebastien/sextant-mcp/releases/download/${SEXTANT_VERSION}/sextant-${SEXTANT_VERSION}-${ARCH}.tar.gz"

shasum -a 256 -c <(curl -sSL \
  "https://github.com/kylebastien/sextant-mcp/releases/download/${SEXTANT_VERSION}/SHA256SUMS" \
  | grep "${ARCH}")

tar -xzf sextant.tar.gz
install -m 0755 sextant sextant-mcp ~/.local/bin/
```

## Homebrew

A tap is planned for the v0.1.0 release.

```sh
brew install kylebastien/sextant/sextant
```

This will install both binaries.

## From source

You'll need [Rust 1.75 or newer](https://rustup.rs/). Clone the repo and
install the two binaries with `cargo`:

```sh
git clone https://github.com/kylebastien/sextant-mcp
cd sextant-mcp
cargo install --path crates/sextant-cli
cargo install --path crates/sextant-mcp
```

Both binaries land in `~/.cargo/bin`, which `rustup` puts on `PATH` by
default.

To build the workspace without installing:

```sh
cargo build --workspace --release
./target/release/sextant --help
./target/release/sextant-mcp --help
```

## Verify the install

```sh
sextant --version
sextant-mcp --version
```

If `command not found`, check that the install directory (e.g.,
`~/.local/bin`, `~/.cargo/bin`) is on `PATH`:

```sh
echo $PATH | tr ':' '\n' | grep -E '\.local/bin|\.cargo/bin'
```

## Next steps

- [Run the quickstart](/sextant-mcp/getting-started/quickstart/) to grade
  your first repo.
- [Bootstrap a config with `sextant init`](/sextant-mcp/getting-started/init/).
- [Install the Claude Code plugin](/sextant-mcp/plugin/) for in-editor
  grading during the agent loop.

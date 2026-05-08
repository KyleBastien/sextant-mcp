# Sextant

**Code-quality grader for AI-agent workflows.** Deterministic rules and
LLM judges, surfaced inside the agent loop and on every PR.

📖 **[Documentation →](https://kylebastien.github.io/sextant-mcp/)**

One engine, four surfaces:

- **MCP server** — agents call `grade_diff` after each edit and
  self-correct before finishing the turn.
- **GitHub Action** — posts a single review comment on every PR. Gates
  merge on regression — pre-existing findings don't block new work.
- **Claude Code plugin** — skills, slash commands, and hooks that wire
  grading into the edit loop with zero extra configuration.
- **CLI** — `sextant grade` for human, JSON, markdown, or SARIF output.
  Works offline once installed.

## Highlights

- **Built for the inner loop.** Diff-mode grading runs in well under a
  second on a typical change, so an agent can grade after every edit
  without slowing the user down.
- **Deterministic by default.** Seven built-in rules cover file length,
  function length, parameter count, cyclomatic complexity, nesting,
  token duplication, and untested public functions — pure Rust, no
  network calls.
- **LLM rules when you want them.** Author markdown rules evaluated by
  Claude or GPT against individual files. Cached by content hash so
  repeat grades are free.
- **Regression-aware.** PR mode only blocks on findings introduced by
  the change. The repo's existing debt is its problem; the PR's job is
  to not make it worse.
- **Multi-language.** Tree-sitter parsers for Rust, Python, Go, Java,
  TypeScript, TSX, and JavaScript.

## Quickstart

Install the two binaries (release archives, Homebrew tap pending, or
from source):

```sh
cargo install --path crates/sextant-cli
cargo install --path crates/sextant-mcp
```

Bootstrap a config and grade your first repo:

```sh
cd your-repo
sextant init
sextant grade                                 # whole-file mode
sextant grade --diff --base origin/main       # only changed lines
sextant rules list                            # what rules are loaded
```

For full installation options (releases, brew, source) see
[Installation](https://kylebastien.github.io/sextant-mcp/getting-started/installation/).

## Use it inside Claude Code

```text
/plugin marketplace add kylebastien/sextant-mcp
/plugin install sextant@kylebastien/sextant-mcp
```

The plugin registers the MCP server, three skills, three slash
commands, and three hooks that grade after every edit and before the
agent ends its turn. Read the
[plugin guide](https://kylebastien.github.io/sextant-mcp/plugin/) for
the details.

## Use it on every PR

```yaml
# .github/workflows/sextant.yml
name: Sextant
on:
  pull_request:

permissions:
  contents: read
  pull-requests: write

jobs:
  grade:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: kylebastien/sextant-mcp/action@v0.1.0
        with:
          fail-on: error
```

Inputs, outputs, baseline-cache behavior, and fork-PR caveats are in
the [Action guide](https://kylebastien.github.io/sextant-mcp/action/).

## Configure

```toml
# .sextant/config.toml
[verdict]
max_errors = 0
max_warns = 50

[size]
file_length_warn = 400
file_length_error = 800
fn_length_warn = 60
fn_length_error = 120

[complexity]
cyclomatic_warn = 10
cyclomatic_error = 20
nesting_warn = 4
nesting_error = 6

[duplication]
min_tokens = 100

[judge]                # LLM-evaluated rules
enabled = true
api_key_env = "ANTHROPIC_API_KEY"
```

Full schema reference:
[Configuration](https://kylebastien.github.io/sextant-mcp/configuration/).

## Layout

```
crates/
  sextant-core/      types: Rule, Finding, Report, Verdict, Evaluator
  sextant-config/    TOML config loader
  sextant-rules/     built-in rule evaluators + rule discovery
  sextant-diff/      git diff acquisition (git2)
  sextant-lang/      tree-sitter parsers (rust, python, go, java, ts, js)
  sextant-judge/     LLM-as-judge providers + cache (Anthropic, OpenAI)
  sextant-engine/    grading orchestration shared by CLI + MCP server
  sextant-cli/       `sextant` binary
  sextant-mcp/       `sextant-mcp` server (stdio + HTTP)
plugin/              Claude Code plugin (skills, slash commands, hooks)
action/              GitHub Action
docs/                Astro Starlight documentation site
```

See [Architecture](https://kylebastien.github.io/sextant-mcp/architecture/)
for the dependency graph and per-crate notes.

## Contributing

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

The repo is graded by Sextant itself (see
`.github/workflows/sextant-grade.yml`). PRs that introduce new errors
will fail the gate; pre-existing findings won't.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.

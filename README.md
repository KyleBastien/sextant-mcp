# Sextant

**Code-quality grader for AI-agent workflows.** Deterministic rules and
LLM judges, surfaced inside the agent loop and on every PR.

📖 **[Documentation →](https://kylebastien.github.io/sextant-mcp/)**

## What is Sextant, and when should I use it?

Sextant is a code-quality grader built for codebases where AI agents
(Claude Code, Cursor, Copilot, …) write a real share of the code.

**Imagine** you want every diff — agent-written or human-written — to
stay within your team's bar for file length, function length,
complexity, duplication, test coverage, and whatever other house
rules you have.

**Usually** you only find out a diff is over the line one of three
ways:

- a teammate catches it in PR review, hours or days later;
- CI lint or format jobs flag it after the agent has already
  finished its turn and moved on;
- you read the diff yourself, decide the function is too long or
  duplicates something, and either fix it by hand or feed the
  complaint back to the agent as a follow-up prompt.

All three are reactive. By the time the feedback lands, the agent's
context is cold and the cleanup falls on you.

**Sextant moves that feedback into the loop.** The same grader runs
in two places:

- **inside the agent**, as an MCP tool and Claude Code hooks — the
  agent grades its own diff after every edit and self-corrects
  before ending the turn;
- **on every PR**, as a GitHub Action that posts a single review
  comment and only blocks merge on regressions, so pre-existing
  debt doesn't gate new work.

One `.sextant/` config drives both. You customize:

- **what counts as a finding** — thresholds for the built-in size,
  complexity, duplication, and untested-public-function rules, plus
  your own markdown rules judged by Claude or GPT against individual
  files;
- **how strict the verdict is** — how many errors or warnings flip a
  grade from `approve` to `request_changes`;
- **what the grader looks at** — just the diff (default; fast,
  ignores legacy debt), the touched files, or the whole repo.

If you're hand-reviewing every agent diff for the same handful of
issues, or watching CI flag the same things over and over, Sextant
is the loop you're missing.

## One engine, five surfaces

- **MCP server** — agents call `grade_diff` after each edit and
  self-correct before finishing the turn.
- **GitHub Action** — posts a single review comment on every PR. Gates
  merge on regression — pre-existing findings don't block new work.
- **Claude Code plugin** — skills, slash commands, and hooks that wire
  grading into the edit loop with zero extra configuration.
- **Editor (LSP)** — `sextant-lsp` powers a VS Code extension (and any
  other LSP client) with live squiggles and hover-to-explain showing the
  full rule body.
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

Install the binaries (release archives, Homebrew tap pending, or from
source):

```sh
cargo install --path crates/sextant-cli
cargo install --path crates/sextant-mcp
cargo install --path crates/sextant-lsp   # editor integration
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
commands, and two hooks that grade after every edit — plus a sample
git pre-commit hook for the hard gate at commit time. Read the
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
`.github/workflows/sextant-grade.yml`). PRs that introduce new findings
at warn or above will fail the gate; pre-existing findings won't.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.

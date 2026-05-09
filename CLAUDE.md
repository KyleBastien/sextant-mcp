# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

The toolchain is pinned via `rust-toolchain.toml` (stable + rustfmt + clippy). MSRV is **1.75**.

```sh
# The three checks CI runs (mirror these locally before pushing).
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked

# Single test by name (substring match across the workspace).
cargo test --workspace -- <test_name>

# Single crate's tests.
cargo test -p sextant-engine

# Build the two release binaries (CLI + MCP server).
cargo build --release --bin sextant --bin sextant-mcp --locked

# Install locally (puts `sextant` and `sextant-mcp` on PATH).
cargo install --path crates/sextant-cli
cargo install --path crates/sextant-mcp
```

CI also runs **shellcheck** on `action/scripts/*.sh` and **yamllint** on `.github/workflows` + `action/action.yml` — touch those files and you must lint them locally with the same tools.

Docs site (Astro Starlight, excluded from the cargo workspace):

```sh
cd docs && npm install && npm run dev   # local preview at :4321
```

## Self-grading: this repo grades itself

Sextant is dogfooded on its own source. **Verdict thresholds in `.sextant/config.toml` are `max_errors = 0`, `max_warns = 0`** — any new finding at warn or above blocks the gate.

`.claude/settings.json` wires three hooks:
- **`SessionStart`** prints loaded rules.
- **`PostToolUse`** (Edit/Write/MultiEdit) runs `sextant grade --diff --working-tree --no-llm` after every edit and feeds findings back as context. Silent on a clean grade.
- **`Stop`** runs the same grade. With `SEXTANT_ENFORCE_ON_STOP=1` (set in `.claude/settings.json`), a `request_changes` verdict **blocks turn-end** and feeds findings back as the reason.

Practical consequence: if you add a finding while editing, the loop will not let you stop until it's clean. Drive every diff to `approve` before ending the turn. The `sextant-self-correct` skill describes the grade → fix → re-grade loop and pass budget.

Escape hatches (use sparingly): `SEXTANT_DISABLE_POST_EDIT=1`, `SEXTANT_DISABLE_STOP=1`, `SEXTANT_DISABLE_SESSION_START=1`.

The strictest built-in to watch: file-length warns at 400 lines, errors at 800. `sextant-engine`'s `lib_tests.rs` was extracted from `lib.rs` specifically to stay under the threshold — follow that pattern rather than relaxing the config.

## Workspace architecture

Nine crates in `crates/`, layered. Edit at the lowest layer that owns the concept; higher layers re-export.

```
sextant-core      data model only — Rule, Finding, Report, Verdict, Evaluator trait,
                  SourceFile, VerdictThresholds. No I/O, no logging.
sextant-config    TOML loader (`.sextant/config.toml`) + path-exclude globs.
sextant-lang      tree-sitter parsers (rust, python, go, java, ts/tsx, js).
sextant-diff      git diff acquisition via git2: BaseSpec/HeadSpec → DiffSet
                  (changed_lines per file). `BaseSpec::Auto` = merge-base with
                  origin/main, falling back to HEAD~1.
sextant-rules     Rule discovery + built-in evaluators. Built-in rule markdown
                  is embedded with `rust-embed`; repo-local rules live in
                  `<root>/.sextant/rules/**/*.md`. Evaluator types: `builtin`
                  (dispatched by name in `build_builtin`), `regex`, `llm`.
sextant-judge     LLM-as-judge providers (Anthropic, OpenAI) + content-hash cache.
sextant-engine    Orchestration. `grade()`/`grade_with()` is the single entry
                  point: load config → build judge → load RuleSet → walk files
                  (whole-tree) or compute diff → grade → filter to changed lines
                  if diff mode → compute verdict → return Report. `grade_pr()`
                  wraps it for regression-only PR grading with a baseline cache.
sextant-cli       `sextant` binary. Subcommands: grade, rules (list/explain/check),
                  init. Thin wrapper over the engine.
sextant-mcp       `sextant-mcp` binary. MCP stdio server (default) or HTTP server
                  (`--http <addr>`). Both transports funnel through
                  `handler::handle_line`. Tools: grade_diff, grade_files,
                  list_rules, explain_rule, get_config.
```

`Cargo.toml` is the single source of truth for shared deps — add new ones to `[workspace.dependencies]` and reference with `dep.workspace = true`. The workspace forbids `unsafe_code` and warns on `clippy::all`.

### Two grade modes (important when changing the engine)

`GradeMode::Files` walks the tree and grades whole files. `GradeMode::Diff` runs the full grader on the file contents at HEAD, then **filters findings whose line range doesn't intersect a changed line**. Don't try to make rules diff-aware — that filtering happens once, centrally, in `sextant-engine::filter_to_diff`.

PR mode (`grade_pr` / `sextant grade --pr`) goes further: it grades both the base and head as full reports and returns only findings *new* in head. This is what the GitHub Action uses so pre-existing debt doesn't gate new work.

### Adding a built-in rule

1. New evaluator file under `crates/sextant-rules/src/` implementing `Evaluator`.
2. Embed a markdown file (with `evaluator: { type: builtin, name: <your-name> }` frontmatter) so `rust-embed` picks it up.
3. Wire the name into `build_builtin` in `crates/sextant-rules/src/lib.rs`.
4. If the rule has tunable thresholds, add fields to `sextant-config` and read them via `&config.<section>` in `from_parsed`.

Repo-local regex/LLM rules don't need any Rust code — just drop a markdown file in `.sextant/rules/`. See the `sextant-author-rule` skill for the frontmatter schema.

## Plugin and action

- **`plugin/`** is the Claude Code plugin (skills, slash commands, hooks). It's also the marketplace — `/plugin marketplace add kylebastien/sextant-mcp` points at this repo. Hooks are bash; reload with `/plugin reload sextant` or restart the session.
- **`action/`** is the GitHub Action. `action/scripts/*.sh` is shellchecked in CI. The dogfood workflow is `.github/workflows/sextant-grade.yml` — it currently builds from source pending the `v0.1.0` release tag.

`.mcp.json` registers the local `sextant-mcp` binary as an MCP server for Claude Code; `enableAllProjectMcpServers: true` in `.claude/settings.json` opts in automatically.

# Sextant — Claude Code plugin

Bundles [Sextant](https://github.com/kylebastien/sextant-mcp) into a
Claude Code session: the MCP server, three skills the agent
auto-loads, three slash commands, and a sample git pre-commit hook
that blocks commits on a dirty grade.

## Install

You need the `sextant` and `sextant-mcp` binaries on `PATH`. Either:

- Install a release build (recommended):
  `brew install kylebastien/sextant/sextant` once the tap exists, or
  download the matching archive from
  <https://github.com/kylebastien/sextant-mcp/releases>.
- Or build from source: `cargo install --path crates/sextant-cli &&
  cargo install --path crates/sextant-mcp` from a checkout.

Then, from a Claude Code session:

```
/plugin marketplace add kylebastien/sextant-mcp
/plugin install sextant@kylebastien/sextant-mcp
```

The plugin lives at `plugin/` in this repo, so the marketplace is the
repo itself.

## What the plugin does

### MCP server

Registers `sextant-mcp` as an MCP stdio server. Tools surfaced to the
agent: `grade_diff`, `grade_files`, `list_rules`, `explain_rule`,
`get_config`. See the main README for tool semantics.

### Skills (auto-loaded)

- **`sextant-grade`** — when to call `grade_diff` vs `grade_files`,
  how to read the report, severity meanings.
- **`sextant-self-correct`** — the grade → fix → re-grade loop, with
  a 3-pass budget and rules for backing out regressions.
- **`sextant-author-rule`** — frontmatter schema for `.sextant/rules/`,
  evaluator types (regex / llm), validation flow.

Skills are markdown — read them directly under `plugin/skills/`. The
plugin host injects them into the agent's context when their
descriptions match the user's request.

### Slash commands

| Command | What it does |
|---|---|
| `/sextant-grade [paths]` | Grade the working tree (or specified paths) and summarize. |
| `/sextant-init` | Run `sextant init` in the current repo. |
| `/sextant-explain <rule-id>` | Print the markdown body for a rule. |

### Git pre-commit hook

The plugin does **not** wire any Claude Code hooks
(`SessionStart`, `PostToolUse`, `Stop`). Earlier versions did — they
produced dead-end loops and pushed feedback into the wrong place. The
right integration point is `git commit`: the gate runs once per
commit instead of once per keystroke, and a failing grade aborts the
commit outright.

The agent still grades on demand via the MCP server (`grade_diff`,
`grade_files`) and the `sextant-grade` / `sextant-self-correct`
skills tell it when. The pre-commit hook catches anything the agent
(or you) missed.

A sample script lives at `plugin/hooks/pre-commit.sh`. It runs:

```bash
sextant grade --diff --working-tree --no-llm --fail-on warn
```

…and exits non-zero on any warn or error, so `git commit` aborts when
the gate fires.

**Install (symlink from the plugin checkout):**

```bash
ln -sf ../../plugin/hooks/pre-commit.sh .git/hooks/pre-commit
```

…or copy it if you'd rather not depend on the symlink:

```bash
cp plugin/hooks/pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

**Install via [husky](https://typicode.github.io/husky/):**

```bash
npx husky add .husky/pre-commit \
  "sextant grade --diff --working-tree --no-llm --fail-on warn"
```

**Install via the [pre-commit framework](https://pre-commit.com):**

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: sextant
        name: sextant
        entry: sextant grade --diff --working-tree --no-llm --fail-on warn
        language: system
        pass_filenames: false
```

**Tuning:**

- `--fail-on error` — only `error`-severity findings block; warns are
  advisory.
- `--fail-on never` — hook prints findings but never blocks. Useful
  while calibrating new rules.
- Drop `--no-llm` to include LLM-evaluated rules (slower, needs API
  keys).

The script ships **without** an env-var escape hatch — there is no
opt-out flag to flip. If the gate fires, fix the findings or
calibrate the rules.

## Authoring

Skills live at `plugin/skills/<name>/SKILL.md`, commands at
`plugin/commands/<name>.md`, the pre-commit script at
`plugin/hooks/pre-commit.sh`. Each is plain markdown or bash — no
compile step. After editing skills or commands, reload the plugin
(`/plugin reload sextant`) or restart the session.

See `plugin/skills/sextant-author-rule/SKILL.md` for the rule-file
schema.

## Troubleshooting

**"sextant: command not found" when committing.** Your shell `PATH`
doesn't include the install dir. Either install sextant to a
directory already on `PATH` (`~/.local/bin`, `/usr/local/bin`) or add
the install dir to your shell rc so commits launched from `git`
inherit it.

**`HEAD~1` errors on a fresh repo.** The diff grade needs a base
commit. The first commit of a repo has no `HEAD~1`; Sextant returns
a friendly "no default base" error and the hook exits silently.

**Pre-commit hook blocked the commit.** Fix the findings. The gate is
strict by design and has no escape hatch — if it fires repeatedly on
something that isn't a real problem, calibrate the rules (lower
severity, narrow the regex, tighten the LLM prompt) rather than
trying to skip the gate.

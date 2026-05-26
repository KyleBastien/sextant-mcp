# Sextant — Claude Code plugin

Bundles [Sextant](https://github.com/kylebastien/sextant-mcp) into a
Claude Code session: the MCP server, three skills the agent
auto-loads, three slash commands, and two hooks that turn grading
into a live signal during the edit loop — plus a sample git
pre-commit hook for the hard gate.

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
repo itself. After install, restart the session to pick up hooks.

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

### Hooks

The hooks are the reason this is a plugin and not just an MCP server.

#### `SessionStart`

Prints a one-line summary of currently loaded rules so the agent knows
what it's being graded against. No-op when Sextant isn't on `PATH` or
the repo has no rules.

#### `PostToolUse` (Edit / Write / MultiEdit)

After every edit, runs `sextant grade --diff --working-tree --no-llm`
in the background and feeds findings back to the agent as additional
context. Stays silent on a clean grade. `--no-llm` keeps it fast and
offline; explicit grades via the slash command can opt back in.

Skipped automatically when `.sextant/` doesn't exist — new repos
don't pay the cost on every edit.

### Git pre-commit hook (recommended hard gate)

The plugin no longer ships a Claude Code `Stop` hook. The
agent's `PostToolUse` feedback is a *soft* signal: useful for
self-correction during the edit loop, but easy to talk past if you're
moving fast. The hard gate belongs at `git commit` time, where it can
block the commit until the diff is clean.

A sample script lives at `plugin/hooks/pre-commit.sh`. It runs
`sextant grade --diff --working-tree --no-llm --fail-on warn` and
exits non-zero on any warn or error — `git commit` aborts when the
gate fires.

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
npx husky add .husky/pre-commit "sextant grade --diff --working-tree --no-llm --fail-on warn"
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
- `SEXTANT_SKIP_PRECOMMIT=1` short-circuits the script to a no-op when
  you need to bypass it for a session.
- `git commit --no-verify` is the per-commit escape hatch — use
  sparingly.

The post-edit Claude hook still runs during the edit loop, so the
agent gets feedback long before the commit attempt. The pre-commit
hook is the safety net that catches anything the agent (or you)
missed.

## Disabling pieces

Plugin hooks are all-or-nothing in the manifest, so to skip individual
Claude hooks rename them inside your fork or set guards inline. The
simplest escape hatch:

```bash
# Disable post-edit grading without uninstalling the plugin.
export SEXTANT_DISABLE_POST_EDIT=1
```

(The hook scripts respect that env var as a no-op short-circuit. Same
pattern for `SEXTANT_DISABLE_SESSION_START=1` and
`SEXTANT_SKIP_PRECOMMIT=1` for the git pre-commit hook.)

## Authoring

Skills live at `plugin/skills/<name>/SKILL.md`, commands at
`plugin/commands/<name>.md`, hooks at `plugin/hooks/*.sh`. Each is
plain markdown or bash — no compile step. After editing, reload the
plugin (`/plugin reload sextant`) or restart the session.

See `plugin/skills/sextant-author-rule/SKILL.md` for the rule-file
schema.

## Troubleshooting

**"sextant: command not found" in hook output.** The plugin host
inherits your shell `PATH`. Either install sextant to a directory
already on `PATH` (`~/.local/bin`, `/usr/local/bin`) or add the
install dir explicitly.

**Hooks fire but produce no output.** That's the silent-on-clean
behavior — they only speak when there are findings. Run
`sextant grade --diff --working-tree` manually to confirm.

**`HEAD~1` errors on a fresh repo.** The diff hook needs a base
commit. The first commit of a repo has no `HEAD~1`; Sextant returns
a friendly "no default base" error and the hook exits silently.

**Pre-commit hook blocked the commit and I need to land something
dirty.** Bypass once with `git commit --no-verify`, or disable the
hook for a session with `SEXTANT_SKIP_PRECOMMIT=1`. The post-edit
Claude hook still runs, so you'll see findings during the next
session even if you bypass the commit gate.

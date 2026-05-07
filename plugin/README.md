# Sextant — Claude Code plugin

Bundles [Sextant](https://github.com/kylebastien/sextant-mcp) into a
Claude Code session: the MCP server, three skills the agent
auto-loads, three slash commands, and three hooks that turn grading
into a live signal during the edit loop.

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

#### `Stop`

Before the agent ends its turn, runs the same `grade_diff` once more.

- **Default (advisory):** surface the verdict and findings as
  context. Don't block stop. The agent can volunteer one more pass.
- **Enforcing:** set `SEXTANT_ENFORCE_ON_STOP=1` in your shell or
  Claude Code env. A `request_changes` verdict now *blocks* the stop
  and feeds the findings back as the reason — Claude continues
  iterating until the verdict is `approve` or it gives up.

Pick advisory if you want Sextant as a code-review companion;
enforcing if you want it as a guardrail.

## Disabling pieces

Plugin hooks are all-or-nothing in the manifest, so to skip individual
hooks rename them inside your fork or set guards inline. The simplest
escape hatch:

```bash
# Disable post-edit grading without uninstalling the plugin.
export SEXTANT_DISABLE_POST_EDIT=1
```

(The hook scripts respect that env var as a no-op short-circuit. Same
pattern for `SEXTANT_DISABLE_STOP=1` and `SEXTANT_DISABLE_SESSION_START=1`.)

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

**Enforce mode blocked stop and the agent is stuck.** Either fix the
findings or unset `SEXTANT_ENFORCE_ON_STOP` for one turn:
```bash
SEXTANT_ENFORCE_ON_STOP=0 claude
```

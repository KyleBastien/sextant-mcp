---
title: Pre-commit hook
description: Use Sextant as a git pre-commit hook to block commits until the diff grades clean.
sidebar:
  order: 4
---

The Sextant plugin doesn't wire any Claude Code hooks
(`SessionStart`, `PostToolUse`, `Stop`). Earlier versions did — they
produced dead-end loops and pushed feedback into the wrong place. The
right integration point is **`git commit`**: the rest of the toolchain
already understands the bypass semantics, and the gate runs once per
commit instead of once per keystroke.

The agent still grades on demand via the MCP server (`grade_diff`,
`grade_files`) and the `sextant-grade` /
[`sextant-self-correct`](/sextant-mcp/plugin/skills/#sextant-self-correct)
skills tell it when. The pre-commit hook catches anything the agent
(or you) missed.

A sample script ships at `plugin/hooks/pre-commit.sh` in the
[sextant-mcp repo](https://github.com/kylebastien/sextant-mcp). It
runs `sextant grade --diff --working-tree` and aborts the commit on
any finding — `git commit --no-verify` is the standard bypass.

## What the sample script does

```bash
sextant grade --diff --working-tree --no-llm --fail-on warn
```

- `--diff --working-tree` — grade only changed lines against the
  merge-base with `origin/main`, taking the working tree as head.
- `--no-llm` — skip LLM-evaluated rules so the gate is fast and
  offline. Drop the flag if you want the heavier rules in the gate.
- `--fail-on warn` — exit non-zero on any warn or error finding,
  matching a strict gate. See [tuning](#tuning) below.

The script also short-circuits when `sextant` isn't on `PATH`, when
the repo has no `.sextant/` directory, or when `SEXTANT_SKIP_PRECOMMIT=1`
is set — none of those should block a commit.

## Installing

### Plain git hook

From the repo root:

```bash
ln -sf ../../plugin/hooks/pre-commit.sh .git/hooks/pre-commit
```

The symlink follows future edits to the script. If you'd rather copy:

```bash
cp plugin/hooks/pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### husky

```bash
npx husky add .husky/pre-commit \
  "sextant grade --diff --working-tree --no-llm --fail-on warn"
```

### pre-commit framework

`.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: sextant
        name: sextant
        entry: sextant grade --diff --working-tree --no-llm --fail-on warn
        language: system
        pass_filenames: false
```

`pass_filenames: false` is important — sextant resolves its own file
set from the diff; the framework shouldn't pass touched paths in.

### lefthook

```yaml
# lefthook.yml
pre-commit:
  commands:
    sextant:
      run: sextant grade --diff --working-tree --no-llm --fail-on warn
```

## Tuning

| Flag | When to use |
|---|---|
| `--fail-on warn` | Strict gate (default in the sample script). Any warn blocks the commit. |
| `--fail-on error` | Errors block, warns are advisory. Good while a rule set is still maturing. |
| `--fail-on never` | Hook prints findings but never blocks. Useful for the very first week, before you've calibrated. |
| Drop `--no-llm` | Include LLM-evaluated rules. Slower; needs API keys in the shell that runs the hook. |

The verdict still depends on `[verdict]` thresholds in
`.sextant/config.toml` — `--fail-on` only controls how the CLI maps
the report to an exit code.

## Bypassing

- **Per commit:** `git commit --no-verify`. The standard escape hatch
  for any git hook.
- **Per session:** `export SEXTANT_SKIP_PRECOMMIT=1`. The sample script
  reads this and exits 0 immediately.
- **Permanent off:** `chmod -x .git/hooks/pre-commit` or remove the
  symlink.

Use bypasses sparingly. The whole point is to make the strict path
the default.

## Combining with CI

The pre-commit hook is the *local* gate. Pair it with the
[GitHub Action](/sextant-mcp/action/) for the same gate at the PR
level. The Action's regression mode means CI only blocks on *new*
findings — so a clean local pre-commit loop produces clean CI, and
existing debt doesn't gate new work.

## Troubleshooting

**The hook is too slow.** Make sure `--no-llm` is set. If the wait is
still too long, profile with `time sextant grade --diff --working-tree`
and look for an expensive rule. Heavy LLM-evaluated rules belong in
CI, not in the commit hook.

**The hook flags lines I didn't touch.** Diff mode filters findings to
changed lines, but a rule that fires on the *file* (size, complexity,
duplication) still surfaces because the file changed. That's working
as intended — those rules want you to refactor when you touch a fat
file. See [Self-grading](/sextant-mcp/configuration/#self-grading)
for how the Sextant repo handles this.

**The hook fires on every commit because the repo has pre-existing
warnings.** Switch to `--fail-on error` until you've cleaned up, or
seed the baseline by running `sextant grade --fail-on never` once and
committing the report.

## See also

- [Skills](/sextant-mcp/plugin/skills/) — the auto-loaded skills that
  prompt the agent to grade.
- [`sextant grade`](/sextant-mcp/cli/grade/) — the underlying
  command.
- [GitHub Action](/sextant-mcp/action/) — the PR-level gate.

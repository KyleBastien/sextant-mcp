---
title: Enforcing mode
description: Turn the Stop hook into a guardrail that blocks turns until the verdict flips to approve.
sidebar:
  order: 5
---

By default, the [Stop hook](/sextant-mcp/plugin/hooks/#stop) is
**advisory**: it grades the diff before the agent ends its turn,
surfaces the findings, and lets the turn end. The agent (and the
user) decide whether to act.

**Enforcing mode** flips that default: a `request_changes` verdict
blocks the stop and feeds findings back as the reason the turn can't
end. The agent has to fix things — or give up — before handing
control back to you.

## Turn it on

```sh
export SEXTANT_ENFORCE_ON_STOP=1
```

Set it in your shell rc, in Claude Code's env config, or per-session
when you launch the CLI.

## What changes

| Aspect | Advisory | Enforcing |
|---|---|---|
| Stop hook on `request_changes` | Surfaces findings, allows stop. | Blocks stop, feeds findings back as the reason. |
| Agent behaviour | Volunteers one more pass at most. | Iterates until verdict is `approve` (or it gives up). |
| Token cost | Cheap — one extra grade. | Higher — every fix consumes a turn. |
| User feel | Helpful nudge. | Strict guardrail. |
| Stuck turns | Rare. | Possible — agent loops on a finding it can't fix. |

## When to use it

**Good fit:**

- Repos with low-noise rules — false positives are rare, so blocked
  turns are almost always real issues.
- Teams that want CI-equivalent gating during interactive editing.
- Codebases under heavy refactor where you want the agent to stay
  honest about regressions.

**Bad fit:**

- Repos with chatty LLM rules — frequent false positives mean
  frequent dead-end loops.
- Exploratory sessions where you want the agent to draft something
  rough before tightening.
- New repos where the rules haven't been calibrated yet.

A reasonable progression: start advisory, calibrate rules so warns and
errors mean what you want, then turn enforcing on.

## Bypassing for one turn

If enforcing mode blocks a stop and you want to override:

```sh
SEXTANT_ENFORCE_ON_STOP=0 claude
```

Or unset for the rest of the shell:

```sh
unset SEXTANT_ENFORCE_ON_STOP
```

The agent still grades on stop (if the hook isn't otherwise disabled),
but a `request_changes` verdict is advisory again.

## Failure modes to watch for

**The agent loops on a single finding.** Usually because the rule is
too strict or the LLM rule is hallucinating. Check the rule body with
`sextant rules explain <id>`; consider lowering severity or
disabling.

**The agent makes the situation worse.** A pass that *adds* findings
is a regression. The
[`sextant-self-correct` skill](/sextant-mcp/plugin/skills/#sextant-self-correct)
tells the agent to back out regressions, but it isn't perfect —
review the diff before accepting.

**The agent gives up on stop.** With most models, after three or four
failed fix attempts the agent will surface "I can't fix this" and stop
anyway. That's fine — you'll see the findings, decide whether to fix
manually or accept.

## Combining with CI

Enforcing mode is effectively a local pre-commit guard. Pair it with
the [GitHub Action](/sextant-mcp/action/) for the same gate at the PR
level. The Action's regression mode means CI only blocks on *new*
findings — so a clean local enforcing-mode loop produces clean CI.

## Disable the Stop hook entirely

```sh
export SEXTANT_DISABLE_STOP=1
```

Overrides `SEXTANT_ENFORCE_ON_STOP` and skips the hook altogether. The
post-edit hook still runs — you keep mid-edit feedback, lose the
end-of-turn check.

## See also

- [Hooks](/sextant-mcp/plugin/hooks/) — the full hook reference.
- [Skills → sextant-self-correct](/sextant-mcp/plugin/skills/#sextant-self-correct) —
  the loop the agent follows.
- [Verdict](/sextant-mcp/concepts/verdict/) — what `request_changes`
  means.

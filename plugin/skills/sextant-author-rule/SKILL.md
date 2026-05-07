---
name: sextant-author-rule
description: |
  Use when the user wants to add a new Sextant rule, codify a code-
  review preference, or migrate a hand-grep into a permanent check.
  Covers the rule frontmatter schema, evaluator types, and where rule
  files live.
---

# Authoring Sextant rules

Repo-local rules live at `.sextant/rules/<name>.md`. The file is
markdown with YAML frontmatter — same shape as the built-in rules
embedded in `crates/sextant-rules/rules/builtin/`.

## Frontmatter schema

```yaml
---
id: project.no-todo                # required, dotted, globally unique
name: "No TODO comments"           # required, human-readable
description: "One-liner."          # required
severity: warn                     # info | warn | error
category: style                    # complexity | size | duplication |
                                   # tests | reliability | style |
                                   # security | docs | { custom: "<name>" }
scope: file                        # diff | file | repo
languages: [rust, python]          # omit = all languages
evaluator:                         # see below
  type: regex
  pattern: '\.unwrap\('
  exclude_paths: ["**/tests/**"]
enabled: true                      # default true
overrides: []                      # rule ids this rule disables
tags: [rust, panics]
---
```

## Evaluator types

### `regex` — line-by-line pattern match

Cheapest option. Good for "no `unwrap()` in prod", forbidden imports,
and similar lexical checks.

```yaml
evaluator:
  type: regex
  pattern: '\.unwrap\(' # standard Rust regex syntax
  exclude_paths: ["**/tests/**", "**/*_test.rs"]
```

### `llm` — LLM-as-judge

Use the rule body as the prompt. Placeholders `{{path}}`, `{{code}}`,
`{{rule.id}}` get substituted at evaluation time. Requires `[judge]`
config in `.sextant/config.toml` and the corresponding API key in env.

```yaml
evaluator:
  type: llm
  model: claude-sonnet-4-6        # optional; falls back to [judge].model
  max_tokens: 1024                # optional
  temperature: 0.0                # optional
  exclude_paths: ["**/tests/**"]
```

The body should explain to the LLM what to look for and ask for
findings as JSON — the schema is enforced via tool-use, but a good
prompt still helps.

### `builtin` — Rust evaluator

Reserved for built-in rules shipped in the binary. Don't use this in
repo-local rules.

## After authoring

1. Validate the file syntax: `sextant rules check .sextant/rules/<name>.md`.
2. Confirm it loads: `sextant rules list | grep <id>`.
3. Try it: `sextant grade` and look for the new rule in the report.
4. Read its docs back: `sextant rules explain <id>`.

## Style

- The body markdown is shown by `explain_rule`. Treat it as user-
  facing docs: explain *why* the rule exists and how to fix a finding.
- Keep severity calibrated. `error` should block real bugs; `warn` is
  for "fix when convenient"; `info` is informational.
- For LLM rules, write the prompt to ask for *concrete* findings tied
  to specific lines. Vague prompts produce vague output.

## Override a built-in rule

Repo-local rules with the same `id` as a built-in replace it. To turn
one off entirely, write a stub:

```yaml
---
id: builtin.size.fn-length
name: "(disabled)"
description: "x"
severity: info
category: size
enabled: false
evaluator: { type: regex, pattern: "(?!)" }
---
```

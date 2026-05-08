---
description: Print the full markdown docs for a Sextant rule by id.
argument-hint: "<rule-id>"
allowed-tools: ["mcp__sextant__explain_rule", "mcp__sextant__list_rules"]
---

Look up the rule whose id is `$ARGUMENTS` via
`mcp__sextant__explain_rule`.

If the id doesn't exist (the tool returns an error / `isError: true`),
fall back to `mcp__sextant__list_rules` and offer the closest matches.

Otherwise, render the rule's markdown body as-is. The body is the
authoritative documentation for the rule — it explains *why* the rule
exists, what trips it, and how to fix a finding.

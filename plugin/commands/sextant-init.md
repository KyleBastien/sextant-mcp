---
description: Bootstrap a `.sextant/` directory in the current repo (config + sample rule).
allowed-tools: ["Bash"]
---

Run `sextant init` in the current working directory. If `.sextant/`
already exists, the command will skip files that are already present
unless the user passes `--force`.

After it runs:

1. Show the user what files were created (look at the command output).
2. Mention they can now run `sextant grade` to grade the repo, and
   `sextant rules list` / `sextant rules explain <id>` to inspect rules.
3. If the user wants to author their own rule, suggest invoking the
   `sextant-author-rule` skill (or the `/sextant-explain` slash command
   to read an existing rule's body as a template).

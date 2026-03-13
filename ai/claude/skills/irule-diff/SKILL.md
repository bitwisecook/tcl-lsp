---
name: irule-diff
description: >
  Compare two iRule versions and explain the semantic differences,
  security implications, performance changes, and breaking changes.
allowed-tools: Bash, Read
---

# iRule Diff

Compare two iRule versions and analyse the differences.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read both iRule files (the user should provide two file paths)
3. Run context analysis on both files:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py context $FILE_A
   uv run --no-dev python ai/claude/tcl_ai.py context $FILE_B
   ```
4. Compare the two versions and explain:
   - **Semantic changes** — What changed in behaviour (not just line diffs)?
   - **Events** — Any events added, removed, or reordered?
   - **Security implications** — Do the changes introduce or fix security issues?
   - **Performance implications** — Any changes to hot-path efficiency?
   - **Breaking changes** — Could these changes affect traffic handling?
5. If the user asked a specific question about the diff, focus on that

## Output format

Focus on what matters operationally. Be concise. Use headings for each
analysis section.

$ARGUMENTS

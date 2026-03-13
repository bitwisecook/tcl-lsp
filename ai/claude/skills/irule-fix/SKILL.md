---
name: irule-fix
description: >
  Fix issues in an iRule using LSP diagnostics. Runs an iterative loop:
  analyse, fix issues, re-analyse until clean or stable.
allowed-tools: Bash, Read, Edit
---

# iRule Fix

Fix diagnostics in an iRule iteratively using LSP analysis.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file to fix
3. Run diagnostics to find current issues:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
4. If no errors or warnings, report the file is clean
5. Otherwise, fix the issues in the file using the Edit tool:
   - Follow the security rules and best practices from the domain knowledge
   - Preserve the iRule's behaviour — only fix what the diagnostics flag
   - Fix all issues in a single pass if possible
6. After editing, re-run diagnostics to verify:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
7. If issues remain, fix them and re-run (up to 5 iterations)
8. Report the final state: what was fixed, what remains (if any)

## Important

- Only fix issues flagged by the analyser — do not refactor unrelated code
- Preserve the original behaviour and intent
- If an issue cannot be auto-fixed (e.g. requires architectural change), explain why

$ARGUMENTS

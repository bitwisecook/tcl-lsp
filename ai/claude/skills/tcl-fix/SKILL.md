---
name: tcl-fix
description: >
  Fix issues in a Tcl file using LSP diagnostics. Runs an iterative loop:
  analyse, fix issues, re-analyse until clean or stable.
allowed-tools: Bash, Read, Edit
---

# Tcl Fix

Fix diagnostics in a Tcl file iteratively using LSP analysis.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md`
2. Read the Tcl file to fix
3. Run diagnostics to find current issues:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
4. If no errors or warnings, report the file is clean
5. Otherwise, fix the issues using the Edit tool:
   - Follow the safety and best practices from the domain knowledge
   - Preserve the script's behaviour — only fix what the diagnostics flag
6. After editing, re-run diagnostics to verify:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
7. If issues remain, fix them and re-run (up to 5 iterations)
8. Report the final state

## Important

- Only fix issues flagged by the analyser — do not refactor unrelated code
- Preserve the original behaviour and intent

$ARGUMENTS

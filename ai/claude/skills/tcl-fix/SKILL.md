---
name: tcl-fix
description: "Fix issues in a Tcl file using LSP diagnostics. Runs an iterative analyse-fix-reanalyse loop until clean or stable. Use when fixing Tcl errors, resolving .tcl file warnings, auto-fixing Tcl lint issues, or cleaning up Tcl diagnostics."
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
4. If the tool fails (e.g. file not found), report the error clearly and suggest fixes
5. If no errors or warnings, report the file is clean
6. Otherwise, fix the issues using the Edit tool:
   - Follow the safety and best practices from the domain knowledge
   - Preserve the script's behaviour -- only fix what the diagnostics flag
7. After editing, re-run diagnostics to verify:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
8. If issues remain, fix them and re-run (up to 5 iterations)
9. If issues persist after 5 iterations, report what was fixed and what remains unresolved
10. Report the final state

## Diagnostic codes reference

See `docs/generated/diagnostic_tables.md` for the full auto-generated table of all diagnostic codes.

## Important

- Only fix issues flagged by the analyser -- do not refactor unrelated code
- Preserve the original behaviour and intent

$ARGUMENTS

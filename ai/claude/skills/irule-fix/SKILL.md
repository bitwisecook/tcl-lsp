---
name: irule-fix
description: "Fix issues in an F5 iRule using LSP diagnostics. Runs an iterative analyse-fix-reanalyse loop until clean or stable. Use when fixing iRule errors, resolving iRule warnings, auto-fixing iRule lint issues, or cleaning up F5 iRule diagnostics."
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
4. If the tool fails (e.g. file not found), report the error clearly and suggest fixes
5. If no errors or warnings, report the file is clean
6. Otherwise, fix the issues in the file using the Edit tool:
   - Follow the security rules and best practices from the domain knowledge
   - Preserve the iRule's behaviour -- only fix what the diagnostics flag
   - Fix all issues in a single pass if possible
7. After editing, re-run diagnostics to verify:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
8. If issues remain, fix them and re-run (up to 5 iterations)
9. If issues persist after 5 iterations, report what was fixed and what remains unresolved
10. Report the final state: what was fixed, what remains (if any)

## Diagnostic codes reference

See `docs/generated/diagnostic_tables.md` for the full auto-generated table of all diagnostic codes.

## Important

- Only fix issues flagged by the analyser -- do not refactor unrelated code
- Preserve the original behaviour and intent
- If an issue cannot be auto-fixed (e.g. requires architectural change), explain why

$ARGUMENTS

---
name: irule-fix
description: "Fix issues in an F5 iRule using LSP diagnostics. Runs an iterative analyse-fix-reanalyse loop until clean or stable. Use when fixing iRule errors, resolving iRule warnings, auto-fixing iRule lint issues, or cleaning up F5 iRule diagnostics."
allowed-tools: mcp__tcl-lsp__analyze, Read, Edit
---

# iRule Fix

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. Call `mcp__tcl-lsp__analyze` with the contents as `source`. On a tool
   error report it; if there are no errors or warnings, report the file is
   clean.
3. Fix only what the diagnostics flag, in one pass where possible, following
   the security rules and preserving behaviour and intent. Codes:
   `docs/generated/diagnostic_codes.md`.
4. Re-read and re-analyse; repeat up to 5 iterations.
5. Report what was fixed and what remains, with a reason for anything that
   cannot be auto-fixed (e.g. needs an architectural change).

$ARGUMENTS

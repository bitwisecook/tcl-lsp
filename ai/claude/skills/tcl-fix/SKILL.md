---
name: tcl-fix
description: "Fix issues in a Tcl file using LSP diagnostics. Runs an iterative analyse-fix-reanalyse loop until clean or stable. Use when fixing Tcl errors, resolving .tcl file warnings, auto-fixing Tcl lint issues, or cleaning up Tcl diagnostics."
allowed-tools: mcp__tcl-lsp__analyze, Read, Edit
---

# Tcl Fix

## Steps

1. Read `../_prompts/tcl_system.md`, then the file.
2. Call `mcp__tcl-lsp__analyze` with the contents as `source`. On a tool
   error report it; if there are no errors or warnings, report the file is
   clean.
3. Fix only what the diagnostics flag, preserving behaviour and intent,
   using Edit. Codes: `docs/generated/diagnostic_codes.md`.
4. Re-read and re-analyse; repeat up to 5 iterations.
5. Report what was fixed and what remains, with a reason for anything that
   cannot be auto-fixed (e.g. needs an architectural change).

$ARGUMENTS

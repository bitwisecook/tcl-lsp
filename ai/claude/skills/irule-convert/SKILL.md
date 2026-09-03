---
name: irule-convert
description: "Modernise legacy iRule patterns to current best practices. Detects unbraced expressions, string concatenation for lists, deprecated matchclass, ungated logs, and other convertible patterns. Use when modernising iRules, converting legacy F5 iRule code, upgrading iRule syntax, or applying iRule best practices."
allowed-tools: mcp__tcl-lsp__find-legacy, mcp__tcl-lsp__analyze, Read, Edit
---

# iRule Convert (Modernise)

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. Call `mcp__tcl-lsp__find-legacy` with the contents as `source`. On a tool
   error report it; with no findings, report the iRule already follows best
   practice.
3. Apply the conversions with Edit: `matchclass` → `class match`
   (IRULE2001); unbraced expr → braced (W100); string concat for lists →
   `lappend` (W104); `==` / `!=` on strings → `eq` / `ne` (W110); missing `--`
   → add it (W304); ungated log in a hot event → `CLIENT_ACCEPTED { set
   debug 0 }` plus `if {$debug}` (IRULE5001).
4. Re-read and re-validate with `mcp__tcl-lsp__analyze`; fix new issues, up
   to 3 iterations.
5. Report what was converted and the final validation status.

$ARGUMENTS

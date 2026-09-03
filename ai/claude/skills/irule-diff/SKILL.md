---
name: irule-diff
description: "Compare two iRule versions and explain the semantic differences, security implications, performance changes, and breaking changes. Uses LSP context analysis on both files. Use when comparing iRule versions, diffing F5 iRules, analysing iRule change impact, or reviewing iRule modifications."
allowed-tools: mcp__tcl-lsp__irule_with_context, Read
---

# iRule Diff

## Steps

1. Read `../_prompts/irules_system.md`, then both files.
2. Call `mcp__tcl-lsp__irule_with_context` once per file with its contents
   as `config_text`. If either call fails, compare the sources by hand and
   say LSP analysis was unavailable.
3. Explain, concisely and operationally, with a heading each: semantic
   changes (behaviour, not lines); events added, removed, or reordered;
   security implications; performance implications on the hot path;
   breaking changes to traffic handling. Focus on any specific question
   asked.

$ARGUMENTS

---
name: tcl-explain
description: "Explain what a Tcl script does by breaking down procedures, data flow, and overall structure. Uses LSP analysis for accurate context including call graphs and diagnostic insights. Use when explaining Tcl code, understanding what a .tcl file does, analysing Tcl script structure, or answering questions about Tcl procedures."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__call_graph, Read
---

# Tcl Explain

## Steps

1. Read `../_prompts/tcl_system.md`, then the file.
2. Call `mcp__tcl-lsp__analyze` (diagnostics, symbols, events) and
   `mcp__tcl-lsp__call_graph` (caller→callee graph, roots, leaves) with the
   contents as `source`. If analysis fails (e.g. syntax errors), explain from
   the source alone and say LSP analysis was unavailable.
3. Explain, focusing on any specific question the user asked.

## Output

- **Summary** — one paragraph on what the script does
- **Procedures** — per proc: purpose, parameters, return value (omit if none)
- **Data flow** — how data moves between procs and through control structures
- **Issues** — analyser findings (omit if clean)

$ARGUMENTS

---
name: irule-dataflow
description: "Analyse and visualise data flow in an iRule or Tcl file. Extracts def-use chains, memory aliases, and data-flow edges from the compiler SSA, then generates a Mermaid diagram showing how variables flow through the program. Use when analysing iRule data flow, visualising Tcl variable usage, debugging iRule variable scoping, or understanding def-use chains."
allowed-tools: mcp__tcl-lsp__def_use_chains, mcp__tcl-lsp__memory_aliases, Read
---

# Data Flow Analysis

## Steps

1. Read the file.
2. Call `mcp__tcl-lsp__def_use_chains` and `mcp__tcl-lsp__memory_aliases`
   with the contents as `source`. On a tool error report it and suggest
   fixes.
3. Draw the diagram and write the analysis.

## Mermaid rules

- `flowchart TD`; one subgraph per proc or event handler
- `[var#N]` rectangles for definitions, `{phi}` diamonds for phi nodes,
  `{{alias}}` hexagons for upvar/global aliases
- Green for live definitions, red for dead ones, orange for aliases
- Solid arrows def→use, dashed for phi flow, dotted labelled arrows for
  aliases; lattice values (CONST / OVERDEFINED) and inferred types on
  definition nodes where known

## Output

The diagram in a ```mermaid fence, then: variable flow summary, dead
definitions (optimisation opportunities), alias relationships, phi merge
points, and potential issues (uninitialised reads, unnecessary stores, alias
confusion).

$ARGUMENTS

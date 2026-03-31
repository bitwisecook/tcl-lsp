---
name: irule-dataflow
description: "Analyse and visualise data flow in an iRule or Tcl file. Extracts def-use chains, memory aliases, and data-flow edges from the compiler SSA, then generates a Mermaid diagram showing how variables flow through the program. Use when analysing iRule data flow, visualising Tcl variable usage, debugging iRule variable scoping, or understanding def-use chains."
allowed-tools: Bash, Read
---

# Data Flow Analysis

Analyse data flow in an iRule or Tcl file using compiler SSA analysis.

## Steps

1. Read the iRule/Tcl file
2. Extract def-use chain information:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py def-use $FILE
   ```
3. Extract memory alias information:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py memory-aliases $FILE
   ```
4. If either tool fails (e.g. file not found or parse error), report the error clearly and suggest fixes
5. Using the def-use chains and alias data, generate a Mermaid diagram and analysis

## Mermaid diagram rules

- Use `flowchart TD` (top-down direction)
- Create a **subgraph** for each function/event handler
- Inside each subgraph:
  - Use **rectangle shapes** `[var#N]` for variable definitions
  - Use **diamond shapes** `{phi}` for phi nodes (conditional merge points)
  - Use **hexagon shapes** `{{alias}}` for aliased variables (upvar/global)
  - Use **red styling** for dead definitions (no uses)
  - Use **green styling** for live definitions (has uses)
  - Use **orange styling** for aliased variables
- Connect definitions to uses with arrows:
  - **Solid arrows** for direct def->use flow
  - **Dashed arrows** for phi (conditional) flow
  - **Dotted arrows** with label for alias relationships
- Include lattice values (CONST/OVERDEFINED) on definition nodes where known
- Include type information where inferred

## Output

1. The Mermaid data-flow diagram in a ```mermaid code fence
2. A structured analysis including:
   - **Variable flow summary**: How key variables flow through the program
   - **Dead definitions**: Variables defined but never used (optimisation opportunities)
   - **Alias relationships**: Variables linked via upvar/global/variable
   - **Phi merge points**: Where variable values depend on control flow
   - **Potential issues**: Uninitialised reads, unnecessary stores, alias confusion

$ARGUMENTS

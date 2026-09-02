---
name: irule-diagram
description: "Generate a Mermaid flowchart of an iRule's logic flow. Extracts structured data from the compiler IR and produces a visual diagram with event subgraphs, decision points, and actions. Use when visualising iRule logic, generating F5 iRule flowcharts, diagramming iRule event flow, or documenting iRule architecture."
allowed-tools: mcp__tcl-lsp__diagram, Read
---

# iRule Diagram

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. Call `mcp__tcl-lsp__diagram` with the contents as `source`. On a tool
   error report it and suggest fixes.
3. Draw the flowchart from the structured data (authoritative) with the
   source for reference.

## Mermaid rules

- `flowchart TD`; one subgraph per event handler labelled with the event
  name, in firing order; a non-default priority (not 500) in the label
- `{Decision}` diamonds for `if` / `switch` (the subject in the diamond, the
  pattern or condition on each edge); `[Action]` rectangles for `pool`,
  `HTTP::respond`, `HTTP::redirect`, `HTTP::header`, `log`, …; `(Return)` for
  `return`; `([Loop])` for loops
- Called procs are their own subgraphs linked from the call site
- Meaningful node IDs (`hr_switch`, not `A1`); labels under 40 characters,
  long strings abbreviated with "..."; labels with special characters
  double-quoted

## Output

The diagram in a ```mermaid fence, then two to four paragraphs: firing order
and cross-event data flow, the key decisions and routing, any
security-relevant actions.

$ARGUMENTS

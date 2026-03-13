---
name: irule-diagram
description: >
  Generate a Mermaid flowchart of an iRule's logic flow.
  Extracts structured data from the compiler IR and produces
  a visual diagram with event subgraphs, decision points, and actions.
allowed-tools: Bash, Read
---

# iRule Diagram

Generate a Mermaid flowchart from an iRule using compiler IR analysis.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file
3. Extract the structural flow data:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagram $FILE
   ```
4. Using the structured data (authoritative, from the compiler IR) and the source code (for reference), generate a Mermaid flowchart diagram

## Mermaid diagram rules

- Use `flowchart TD` (top-down direction)
- Create a **subgraph** for each event handler, labeled with the event name
- Inside each subgraph:
  - Use **diamond shapes** `{Decision}` for `switch` and `if` decision points
  - Use **rectangle shapes** `[Action]` for commands like `pool`, `HTTP::respond`, `HTTP::redirect`, `HTTP::header`, `log`, etc.
  - Use **rounded rectangles** `(Return)` for `return` statements
  - Use **stadium shapes** `([Loop])` for loops
  - Connect decision points to their branches with labeled edges (the pattern or condition on the arrow)
- If procedures are called, show them as separate subgraphs linked from the call site
- Keep node labels concise (under 40 characters) — abbreviate long strings with "..."
- Use meaningful node IDs (e.g., `hr_switch` not `A1`)
- Show the event subgraphs in firing order (top to bottom)
- If events have a non-default priority (not 500), mention it in the subgraph label
- For switch statements, show the subject being switched on in the diamond, and label each outgoing edge with the match pattern
- Use double-quoted strings for node labels containing special characters

## Output

1. The Mermaid diagram in a ```mermaid code fence
2. A 2-4 paragraph explanation of what the iRule does, including:
   - Event firing order and cross-event data flow
   - Key decision points and routing logic
   - Any security-relevant actions

$ARGUMENTS

---
name: tcl-explain
description: >
  Explain what a Tcl script does. Breaks down procedures, data flow,
  and overall structure using LSP analysis for accurate context.
allowed-tools: Bash, Read
---

# Tcl Explain

Explain what a Tcl script does using LSP analysis for accurate context.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md`
2. Read the Tcl file the user wants explained
3. Run the analysis tool to get structural context:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py context $FILE
   ```
4. Using the domain knowledge, source code, and analysis output, explain:
   - What each procedure does
   - The data flow and control structure
   - Any issues identified by the analyser
   - The overall purpose of the script
5. If the user asked a specific question, focus on that

$ARGUMENTS

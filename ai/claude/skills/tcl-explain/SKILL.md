---
name: tcl-explain
description: "Explain what a Tcl script does by breaking down procedures, data flow, and overall structure. Uses LSP analysis from tcl_ai.py for accurate context including call graphs and diagnostic insights. Use when explaining Tcl code, understanding what a .tcl file does, analysing Tcl script structure, or answering questions about Tcl procedures."
allowed-tools: Bash, Read
---

# Tcl Explain

Explain what a Tcl script does using LSP analysis for accurate context.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md` for Tcl idioms and best practices
2. Read the Tcl file the user wants explained
3. Run the analysis tool to get structural context:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py context $FILE
   ```
4. If the analysis tool fails (e.g. syntax errors in the file), fall back to manual source reading and note that LSP analysis was unavailable
5. Using the domain knowledge, source code, and analysis output, explain:
   - **Overall purpose**: What the script does at a high level
   - **Procedures**: What each proc does, its parameters, and return values
   - **Data flow**: How data moves between procedures and through control structures
   - **Issues**: Any problems identified by the analyser (warnings, errors, style)
6. If the user asked a specific question, focus the explanation on that area

## Output format

Structure the explanation with clear headings:
- **Summary** -- one-paragraph overview of what the script does
- **Procedures** -- per-proc breakdown (skip if the script has no procs)
- **Data flow** -- how variables and data move through the script
- **Issues** -- any diagnostics from the analyser (omit section if clean)

$ARGUMENTS

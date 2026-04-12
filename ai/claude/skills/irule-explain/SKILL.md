---
name: irule-explain
description: "Explain what an F5 iRule does by breaking down each event handler, describing data flow, noting security concerns, and summarising overall purpose. Uses LSP analysis for accurate context. Use when explaining iRule code, understanding what an iRule does, analysing iRule event handlers, or answering questions about F5 iRule behaviour."
allowed-tools: Bash, Read
---

# iRule Explain

Explain what an iRule does using LSP analysis for accurate context.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file the user wants explained
3. Run the analysis tool to get structural context:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py context $FILE
   ```
4. If the analysis tool fails (e.g. syntax errors in the file), fall back to manual source reading and note that LSP analysis was unavailable
5. Using the domain knowledge, source code, and analysis output, explain:
   - **Overall purpose**: What the iRule does at a high level
   - **Event handlers**: What each event handler does and when it fires
   - **Data flow**: How data flows between events (e.g. variables set in CLIENT_ACCEPTED used in HTTP_REQUEST)
   - **Security concerns**: Any issues identified by the analyser
6. If the user asked a specific question, focus on that while still providing the full context

## Output format

- Use clear headings for each event handler
- Wrap any code references in ```tcl fences
- Highlight security issues prominently
- Note the event firing order and multiplicity (init / once_per_connection / per_request)

$ARGUMENTS

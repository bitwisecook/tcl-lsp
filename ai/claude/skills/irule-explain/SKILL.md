---
name: irule-explain
description: >
  Explain what an iRule does. Breaks down each event handler, describes
  data flow, notes security concerns, and summarises the overall purpose.
  Uses LSP analysis for accurate context.
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
4. Using the domain knowledge, source code, and analysis output, explain:
   - What each event handler does and when it fires
   - The data flow between events (e.g. variables set in CLIENT_ACCEPTED used in HTTP_REQUEST)
   - Any security concerns identified by the analyser
   - The overall purpose of the iRule
5. If the user asked a specific question, focus on that while still providing the full context

## Output format

- Use clear headings for each event handler
- Wrap any code references in ```tcl fences
- Highlight security issues prominently
- Note the event firing order and multiplicity (init / once_per_connection / per_request)

$ARGUMENTS

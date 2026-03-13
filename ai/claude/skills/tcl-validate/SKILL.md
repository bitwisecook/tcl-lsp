---
name: tcl-validate
description: >
  Run full LSP validation on a Tcl file and show a categorized report
  of all issues: errors, security, style, and optimiser suggestions.
allowed-tools: Bash, Read
---

# Tcl Validate

Run full validation and produce a categorized diagnostic report.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md`
2. Read the Tcl file to validate
3. Run the categorized validation:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py validate $FILE
   ```
4. Present the results as a structured report:
   - Group by category (errors, security, style, optimiser)
   - For each issue, explain what it means and how to fix it
   - Provide a summary with total counts per category
5. If the file is clean, say so

$ARGUMENTS

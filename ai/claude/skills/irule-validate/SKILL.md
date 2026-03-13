---
name: irule-validate
description: >
  Run full LSP validation on an iRule and show a categorized report
  of all issues: errors, security, taint, thread safety, performance,
  style, and optimiser suggestions.
allowed-tools: Bash, Read
---

# iRule Validate

Run full validation and produce a categorized diagnostic report.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file to validate
3. Run the categorized validation:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py validate $FILE
   ```
4. Present the results as a structured report:
   - Group by category (errors, security, taint, thread safety, performance, style, optimiser)
   - For each issue, explain what it means and how to fix it using the diagnostic code reference
   - Provide a summary with total counts per category
5. If the file is clean, say so

## Output format

Use headings for each category. For each diagnostic, show:
- The diagnostic code and severity
- The line number and message
- A brief explanation of how to fix it

$ARGUMENTS

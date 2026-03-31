---
name: tcl-validate
description: "Run full LSP validation on a Tcl file and produce a categorised report of all issues: errors, security, style, and optimiser suggestions. Use when validating Tcl code, linting .tcl files, checking Tcl script quality, or running static analysis on Tcl scripts."
allowed-tools: Bash, Read
---

# Tcl Validate

Run full validation on a Tcl file and produce a categorised diagnostic report.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md`
2. Read the Tcl file to validate
3. Run the categorised validation:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py validate $FILE
   ```
4. If the tool fails (e.g. file not found or parse error), report the error clearly and suggest fixes
5. Present the results as a structured report:
   - Group by category (errors, security, style, optimiser)
   - For each issue, explain what it means and how to fix it using the diagnostic code reference
   - Provide a summary with total counts per category
6. If the file is clean, confirm it passes all checks

## Diagnostic codes reference

See `docs/generated/diagnostic_codes.md` for the full auto-generated table of all diagnostic codes with descriptions and defaults.

## Output format

Use headings for each category. For each diagnostic, show:
- The diagnostic code and severity
- The line number and message
- A brief explanation of how to fix it

Example structure:

```
### Errors (2)
- **E100** (line 12): Missing closing brace — add `}` to close the `if` block

### Style (1)
- **W100** (line 5): Unbraced expression — wrap in braces for bytecode compilation

### Summary
- Errors: 2, Style: 1, Total: 3
```

$ARGUMENTS

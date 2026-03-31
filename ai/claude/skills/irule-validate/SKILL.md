---
name: irule-validate
description: "Run full LSP validation on an F5 iRule and produce a categorised report of all issues: errors, security, taint, thread safety, performance, style, and optimiser suggestions. Use when validating iRule code, linting iRules, checking iRule security, analysing F5 iRule diagnostics, or running static analysis on iRules."
allowed-tools: Bash, Read
---

# iRule Validate

Run full validation on an iRule file and produce a categorised diagnostic report.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file to validate
3. Run the categorised validation:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py validate $FILE
   ```
4. If the tool fails (e.g. file not found or parse error), report the error clearly and suggest fixes
5. Present the results as a structured report:
   - Group by category (errors, security, taint, thread safety, performance, style, optimiser)
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
### Errors (1)
- **E001** (line 12): Missing subcommand — add the required subcommand after `string`

### Security (1)
- **T100** (line 5): Tainted data in eval — sanitise user input before passing to eval

### Summary
- Errors: 1, Security: 1, Total: 2
```

$ARGUMENTS

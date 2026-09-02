---
name: irule-validate
description: "Run full LSP validation on an F5 iRule and produce a categorised report of all issues: errors, security, taint, thread safety, performance, style, and optimiser suggestions. Use when validating iRule code, linting iRules, checking iRule security, analysing F5 iRule diagnostics, or running static analysis on iRules."
allowed-tools: mcp__tcl-lsp__validate, Read
---

# iRule Validate

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. Call `mcp__tcl-lsp__validate` with the contents as `source`. On a tool
   error report it and suggest fixes.
3. Report by category (errors, security, taint, thread safety, performance,
   style, optimiser): for each issue the code, severity, line, message, and
   a one-line fix (`docs/generated/diagnostic_codes.md`), then per-category
   totals. A clean file gets an explicit pass.

## Output

```
### Errors (1)
- **E001** (line 12): Missing subcommand — add the required subcommand after `string`

### Security (1)
- **T100** (line 5): Tainted data in eval — sanitise user input before passing to eval

### Summary
- Errors: 1, Security: 1, Total: 2
```

$ARGUMENTS

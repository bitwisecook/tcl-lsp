---
name: tcl-validate
description: "Run full LSP validation on a Tcl file and produce a categorised report of all issues: errors, security, style, and optimiser suggestions. Use when validating Tcl code, linting .tcl files, checking Tcl script quality, or running static analysis on Tcl scripts."
allowed-tools: mcp__tcl-lsp__validate, Read
---

# Tcl Validate

## Steps

1. Read `../_prompts/tcl_system.md`, then the file.
2. Call `mcp__tcl-lsp__validate` with the contents as `source`. On a tool
   error report it and suggest fixes.
3. Report by category (errors, security, style, optimiser): for each issue
   the code, severity, line, message, and a one-line fix
   (`docs/generated/diagnostic_codes.md`), then per-category totals. A clean
   file gets an explicit pass.

## Output

```
### Errors (1)
- **E001** (line 12): Missing subcommand — add the required subcommand after `string`

### Style (1)
- **W100** (line 5): Unbraced expression — wrap in braces for bytecode compilation

### Summary
- Errors: 1, Style: 1, Total: 2
```

$ARGUMENTS

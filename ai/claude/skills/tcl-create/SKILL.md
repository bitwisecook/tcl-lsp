---
name: tcl-create
description: "Create Tcl code from a natural-language description. Generates idiomatic Tcl following best practices, validates it with the LSP analyser, and iterates until diagnostics are clean. Use when creating new Tcl scripts, generating .tcl files from descriptions, writing Tcl procedures, or scaffolding Tcl projects."
allowed-tools: Bash, Read, Write
---

# Tcl Create

Generate Tcl code from a user description, validate with LSP, and iterate until clean.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md` for idiomatic Tcl patterns
2. Generate Tcl code based on the user's description. Requirements:
   - Use braced expressions (`expr {$a + $b}`) and braced script bodies
   - Use list-safe APIs (`list`, `lappend`, `lindex`, `dict`) over manual string concatenation
   - Use `file join` for path construction
   - Use `--` option terminator where needed
   - Include comments for non-obvious logic
3. Write the generated code to a `.tcl` file
4. Validate the generated code:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
5. If there are errors or warnings, fix them and re-validate (up to 5 iterations)
6. If validation still fails after 5 iterations, report remaining issues and explain what could not be resolved
7. Report the final status with a summary of the generated code structure

$ARGUMENTS

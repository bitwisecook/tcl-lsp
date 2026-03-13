---
name: tcl-create
description: >
  Create Tcl code from a description. Generates the code, validates
  it with the LSP, and iterates until clean.
allowed-tools: Bash, Read, Write
---

# Tcl Create

Generate Tcl code from a user description, validate with LSP, and iterate.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md`
2. Generate Tcl code based on the user's description. Requirements:
   - Use braced expressions and braced script bodies
   - Use list-safe APIs (list, lappend, lindex, dict) over manual string concatenation
   - Use `file join` for path construction
   - Use `--` option terminator where needed
   - Include comments for non-obvious logic
3. Write the generated code to a `.tcl` file
4. Validate the generated code:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
5. If there are errors or warnings, fix them and re-validate (up to 5 iterations)
6. Report the final status

$ARGUMENTS

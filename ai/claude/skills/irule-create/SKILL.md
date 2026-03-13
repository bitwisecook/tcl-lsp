---
name: irule-create
description: >
  Create a new iRule from a description. Generates the code, validates
  it with the LSP, and iterates until clean.
allowed-tools: Bash, Read, Write
---

# iRule Create

Generate a new iRule from a user description, validate with LSP, and iterate.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Generate an iRule based on the user's description. Requirements:
   - Use appropriate event handlers (`when` blocks)
   - Follow security best practices (braced expressions, option terminators, no eval with user data)
   - Include comments explaining the logic
   - Use K&R brace style, 4-space indentation
3. Write the generated code to a `.tcl` file (ask the user for the filename, or use a sensible default)
4. Validate the generated code:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
5. If there are errors or warnings, fix them and re-validate (up to 5 iterations)
6. Report the final status: clean or remaining issues

## Output format

Show the final iRule in a ```tcl code fence. Report the validation result
(clean / N issues remaining) and iteration count.

$ARGUMENTS

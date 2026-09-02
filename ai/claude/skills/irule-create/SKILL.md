---
name: irule-create
description: "Create a new F5 iRule from a natural-language description. Generates the code following security best practices, validates with the LSP analyser, and iterates until clean. Use when creating new iRules, generating F5 iRule code from descriptions, writing iRule event handlers, or scaffolding iRule logic."
allowed-tools: mcp__tcl-lsp__analyze, Read, Write
---

# iRule Create

Generate an iRule from a description, validate with the LSP, iterate until
clean.

## Steps

1. Read `../_prompts/irules_system.md`.
2. Generate the iRule: the right `when` handlers, braced expressions, `--`
   terminators, no eval on user data, comments on the logic, K&R braces,
   4-space indent.
3. Write it to a `.tcl` file (ask for a name or pick a sensible default),
   then call `mcp__tcl-lsp__analyze` with the contents as `source`. On a
   tool error report it and adjust the code.
4. Fix errors and warnings and re-validate, up to 5 iterations; then report
   what remains and why.
5. Show the final iRule in a ```tcl fence with the validation result and
   iteration count.

$ARGUMENTS

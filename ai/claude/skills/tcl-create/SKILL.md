---
name: tcl-create
description: "Create Tcl code from a natural-language description. Generates idiomatic Tcl following best practices, validates it with the LSP analyser, and iterates until diagnostics are clean. Use when creating new Tcl scripts, generating .tcl files from descriptions, writing Tcl procedures, or scaffolding Tcl projects."
allowed-tools: mcp__tcl-lsp__analyze, Read, Write
---

# Tcl Create

Generate Tcl from a description, validate with the LSP, iterate until clean.

## Steps

1. Read `../_prompts/tcl_system.md`.
2. Generate the code: braced expressions and bodies, list-safe APIs (`list`,
   `lappend`, `lindex`, `dict`), `file join` for paths, `--` where a value may
   start with `-`, comments only for non-obvious logic.
3. Write it to a `.tcl` file, then call `mcp__tcl-lsp__analyze` with the
   contents as `source`. On a tool error report it and adjust the code.
4. Fix errors and warnings and re-validate, up to 5 iterations; then report
   what remains and why it could not be resolved.
5. Report the final status and a summary of the structure.

$ARGUMENTS

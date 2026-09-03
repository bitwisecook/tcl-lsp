---
name: tcl-refactor
description: "Apply refactorings to Tcl code: extract/inline variables, convert if/elseif chains to switch, convert switch to dict lookup, and brace unbraced expr arguments. Uses the LSP refactoring engine for safe, mechanical transformations. Use when refactoring Tcl scripts, restructuring .tcl code, simplifying Tcl control flow, or applying automated Tcl code transformations."
allowed-tools: mcp__tcl-lsp__refactor, mcp__tcl-lsp__analyze, Read, Edit
---

# Tcl Refactor

## Steps

1. Read the file.
2. Call `mcp__tcl-lsp__refactor` with the contents as `source` and the
   selection (`start_line`, `start_character`, `end_line`, `end_character`)
   covering the code of interest. On a tool error report it.
3. Apply the refactoring the user asked for; otherwise list the available
   ones and ask which.
4. Apply each edit with Edit and say in a sentence why it is safe and what
   it gains.
5. Re-validate with `mcp__tcl-lsp__analyze`; revert any refactoring that
   introduced an issue and say why.

## Refactorings

- Extract variable: expression → named variable
- Inline variable: single-use `set var value` → substituted at the use site
- if/elseif → switch: chain of `$var eq "literal"` tests → `switch -exact`
- switch → dict: arms that set one variable → `dict create` + `dict get`
- Brace expr: `expr "$a + $b"` → `expr {$a + $b}`

$ARGUMENTS

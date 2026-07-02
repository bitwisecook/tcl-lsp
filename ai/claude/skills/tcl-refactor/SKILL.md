---
name: tcl-refactor
description: "Apply refactorings to Tcl code: extract/inline variables, convert if/elseif chains to switch, convert switch to dict lookup, and brace unbraced expr arguments. Uses the LSP refactoring engine for safe, mechanical transformations. Use when refactoring Tcl scripts, restructuring .tcl code, simplifying Tcl control flow, or applying automated Tcl code transformations."
allowed-tools: mcp__tcl-lsp__refactor, mcp__tcl-lsp__analyze, Read, Edit
---

# Tcl Refactor

Apply refactorings to Tcl source code using the LSP refactoring engine.

## Steps

1. Read the Tcl file to refactor
2. Find available refactorings by calling `mcp__tcl-lsp__refactor`, passing the file contents you just read as `source` along with the selection range (`start_line`, `start_character`, `end_line`, `end_character`) covering the code of interest
3. If the tool fails (e.g. parse error), report the error clearly
4. If the user asked for a specific refactoring, apply it.
   Otherwise list the available refactorings and ask which to apply.
5. For each refactoring, apply the edit using the Edit tool
6. Explain in 1-2 sentences why the refactoring is safe and beneficial
7. Validate the refactored file to confirm no regressions by calling `mcp__tcl-lsp__analyze`, passing the refactored file contents as `source`
8. If validation finds new issues, revert the problematic refactoring and explain why

## Available refactorings

- **Extract variable**: select an expression → introduce a named variable
- **Inline variable**: single-use `set var value` → substitute value at use site
- **if/elseif → switch**: chain of `$var eq "literal"` tests → `switch -exact`
- **switch → dict**: every arm sets the same variable → `dict create` + `dict get`
- **Brace expr**: `expr "$a + $b"` → `expr {$a + $b}` (performance + safety)

$ARGUMENTS

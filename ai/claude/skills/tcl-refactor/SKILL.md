---
name: tcl-refactor
description: "Apply refactorings to Tcl code: extract/inline variables, convert if/elseif chains to switch, convert switch to dict lookup, and brace unbraced expr arguments. Uses the LSP refactoring engine for safe, mechanical transformations. Use when refactoring Tcl scripts, restructuring .tcl code, simplifying Tcl control flow, or applying automated Tcl code transformations."
allowed-tools: Bash, Read, Edit
---

# Tcl Refactor

Apply refactorings to Tcl source code using the LSP refactoring engine.

## Steps

1. Read the Tcl file to refactor
2. Run the refactoring scanner to find available refactorings:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py refactor $FILE
   ```
3. If the tool fails (e.g. file not found or parse error), report the error clearly
4. If the user asked for a specific refactoring, apply it.
   Otherwise list the available refactorings and ask which to apply.
5. For each refactoring, apply the edit using the Edit tool
6. Explain in 1-2 sentences why the refactoring is safe and beneficial
7. Validate the refactored file to confirm no regressions:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
8. If validation finds new issues, revert the problematic refactoring and explain why

## Available refactorings

- **Extract variable**: select an expression → introduce a named variable
- **Inline variable**: single-use `set var value` → substitute value at use site
- **if/elseif → switch**: chain of `$var eq "literal"` tests → `switch -exact`
- **switch → dict**: every arm sets the same variable → `dict create` + `dict get`
- **Brace expr**: `expr "$a + $b"` → `expr {$a + $b}` (performance + safety)

$ARGUMENTS

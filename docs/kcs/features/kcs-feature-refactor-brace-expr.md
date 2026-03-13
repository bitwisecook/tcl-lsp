# KCS: feature — Refactor: Brace expr

## Summary

Convert an unbraced `expr "..."` argument to braced `expr {...}` for safety and performance.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

### Editor (all editors via LSP)

Place the cursor on an `expr` command with a double-quoted argument. Trigger code actions and choose **"Brace expr for safety and performance"**.

### MCP

Call the `brace_expr` tool with `source`, `line`, and `character`.

### Claude Code

Use the `refactor` CLI command — it lists brace-expr when the cursor is on an eligible `expr`.

## Before / After

### Before

```tcl
set a 10
set b 20
set sum [expr "$a + $b"]
set product [expr "$a * $b"]
```

### After

```tcl
set a 10
set b 20
set sum [expr {$a + $b}]
set product [expr {$a * $b}]
```

The double-quoted expression arguments are replaced with braced equivalents. This prevents double substitution, avoids code injection risk, and allows the Tcl bytecode compiler to compile the expression at parse time instead of runtime.

## Operational context

Unbraced `expr` arguments are a well-known Tcl anti-pattern: they cause double substitution (the expression string is substituted once by the parser, then again by `expr`), which is both a security risk and a performance penalty. Bracing the argument lets the compiler see the expression structure statically.

The refactoring extracts the raw source text of the quoted argument, strips the quotes, and re-wraps in braces.

## File-path anchors

- `core/refactoring/_brace_expr.py`
- `lsp/features/code_actions.py`

## Failure modes

- Expression already braced (returns `None` — nothing to do).
- Expression contains unbalanced braces inside the string (rare but possible).

## Test anchors

- `tests/test_refactoring.py::TestBraceExpr`

## Samples

- `29-brace-expr-before.tcl` — before bracing
- `29-brace-expr-after.tcl` — after bracing

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)

# KCS: feature — Refactor: Brace expr

> **Audience:** User
> **Type:** Functionality

## Summary

Convert an unbraced `expr "..."` argument to braced `expr {...}` for safety and performance.

## Applies to

all-editors, MCP, Claude skill, refactoring

## How to use

### Editor (all editors via LSP)

Place the cursor on an `expr` command with a double-quoted argument. Trigger code actions and choose **"Brace expr for safety and performance"**.

### MCP

Call the `brace_expr` tool with `source`, `line`, and `character`.

### Claude Code

The `/tcl-refactor` skill calls the `refactor` tool, which lists brace-expr when the cursor is on an eligible `expr`.

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

## Operational context

An unbraced `expr` argument is substituted twice — once by the parser, then
again by `expr`. That is both an injection risk and a performance cost, and it
stops the bytecode compiler seeing the expression. The refactoring takes the
raw source text of the quoted argument, strips the quotes, and re-wraps it in
braces.

## Failure modes

- Expression already braced (returns `None` — nothing to do).
- Expression contains unbalanced braces inside the string (rare but possible).

## Samples

- `29-brace-expr-before.tcl` — before bracing
- `29-brace-expr-after.tcl` — after bracing

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)

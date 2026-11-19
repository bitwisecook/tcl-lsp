# KCS: feature — Refactor: Parenthesise or unparenthesise an expression

> **Audience:** User
> **Type:** Functionality

## Summary

Make every grouping in a braced expression explicit, or remove the
parentheses that operator precedence makes redundant.

## Applies to

all-editors, MCP, Claude skill, refactoring, minifier

## How to use

### Editor (all editors via LSP)

Place the cursor inside a braced expression (`expr {…}`, an `if` or `while`
condition, …). Trigger code actions and choose **"Parenthesise expression by
precedence"** or **"Remove redundant parentheses"**. Each is offered only
when it changes the text.

### MCP

The `refactor` tool lists `parenthesise_expression` and
`remove_redundant_parens` at an eligible position.

## Before / After

```tcl
if {$c & 0xff == 0x81 || ($d)} { … }
```

Parenthesise:

```tcl
if {($c & (0xff == 0x81)) || $d} { … }
```

Remove redundant parentheses:

```tcl
if {$c & 0xff == 0x81 || $d} { … }
```

## Operational context

Both rewrites read the expression with the document's own expression grammar,
so the iRules word operators (`contains`, `starts_with`, `and`, `not`, …)
regroup by their real precedence. Each result is parsed again and offered only
when it gives the same tree, so the refactoring never changes how Tcl
evaluates the expression. The minifier applies the same redundant-parentheses
removal.

Hover an operator or parenthesis to see the parse as a tree.

## Failure modes

- Unbraced or quoted expressions are not offered: their text is substituted
  before `expr` sees it.
- An expression with a parse error is not offered.

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)
- [W154 — operator mix groups differently from how it reads](../codes/kcs-diagnostic-w154-operator-precedence.md)

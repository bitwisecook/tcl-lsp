# KCS: W155 — Non-associative grouping without parentheses

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default, dialect:irule

## Question

How do I find every expression whose meaning depends on operator precedence?

## Why

W155 reports every unparenthesised operand whose other grouping gives a
different result and that W154 does not already report. Only a repeated
associative operator is exempt (`$a + $b + $c`, `$a && $b && $c`, `$a & $b & $c`,
…), as is unary minus under `*`. The rest have a reading convention but still
depend on precedence:

| Written | Parses as | Other grouping |
| --- | --- | --- |
| `10 - 4 + 3` | `(10 - 4) + 3` = 9 | `10 - (4 + 3)` = 3 |
| `12 / 2 * 3` | `(12 / 2) * 3` = 18 | `12 / (2 * 3)` = 2 |
| `2 ** 3 ** 2` | `2 ** (3 ** 2)` = 512 | `(2 ** 3) ** 2` = 64 |
| `-7 % 3` | `(-7) % 3` = 2 | `-(7 % 3)` = -1 |
| `1 ? 5 : 6 + 1` | `1 ? 5 : (6 + 1)` = 5 | `(1 ? 5 : 6) + 1` = 6 |
| `$a == 1 && $b` | `($a == 1) && $b` | `$a == (1 && $b)` |

W155 is a hint and is **off by default**. Turn it on for code that makes
every grouping explicit.

## Symptoms

- With W155 enabled, a hint on the outer operator: "'-' and '+' share a
  precedence level and group left to right, so this parses as
  `(10 - 4) + 3`; …".

## Example that triggers it

```tcl
set total [expr {$base - $discount + $tax}]
```

## Fix

Use the **Parenthesise as parsed** quick fix, or the **Parenthesise
expression by precedence** refactoring for a whole expression.

## How to suppress

W155 is off unless enabled: set `tclLsp.diagnostics.W155` (or the editor's
equivalent), or pass `--enable W155` to the CLI.

## Related

- [KCS codes index](README.md)
- [W154 — operator mix groups differently from how it reads](kcs-diagnostic-w154-operator-precedence.md)
- [Execution evidence: operator groupings](../../../evidence/tcl/expr-precedence-groupings.md)
- [Parenthesise or unparenthesise an expression](../features/kcs-feature-refactor-expression-parens.md)

# KCS: W154 — Operator mix groups differently from how it reads

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default, dialect:irule

## Question

Why is `$c & 0xff == 0x81` flagged?

## Why

An unparenthesised operand is a hazard when the other grouping would give a
different result. W154 reports the mixes that have no shared reading
convention and are a common source of bugs:

| Written | Parses as |
| --- | --- |
| `$c & 0xff == 0x81` | `$c & (0xff == 0x81)` — a comparison under `&`, `^` or `\|` |
| `$a & $b + 1`, `$a & $b << 2` | arithmetic or a shift under a bitwise operator |
| `$a & $b \| $c` | two different bitwise operators |
| `1 << $n - 1` | `1 << ($n - 1)` — arithmetic under a shift |
| `3 < 2 < 1` | `(3 < 2) < 1` — comparisons do not chain |
| `!$x == 5`, `!$x & 1` | `(!$x) == 5`, `(!$x) & 1` |
| `$a \|\| $b && $c` | `$a \|\| ($b && $c)` |
| `$n + 1 ? $a : $b` | `($n + 1) ? $a : $b` |
| `-2 ** 2` | `(-2) ** 2`, which is `4` |

Every Tcl release from 8.4 to 9.1 and Jim group these the same way. The
iRules word operators (`contains`, `starts_with`, `equals`, `and`, `or`,
`not`, …) take part at their own precedence. Every expression is checked:
`expr`, `if`/`elseif`, `while` and `for` conditions, nested `[expr …]`,
math-function arguments, procedure, method and `when` bodies, and literal
expressions written across several words or in quotes.

Other non-associative mixes that do have a convention, such as
`$a + $b * $c`, are W155, which is off by default.

## Symptoms

- A warning on the outer operator: "'==' binds tighter than '&', so this
  parses as `$c & (0xff == 0x81)`; if you meant `($c & 0xff) == 0x81`, add
  parentheses."

## Example that triggers it

```tcl
binary scan $data c type
if {$type & 0xff == 0x81} { puts hit }
```

## Fix

Two quick fixes are offered. **Regroup** writes the other grouping (for a
comparison chain, the conjunction `$a < $b && $b < $c`):

```tcl
if {($type & 0xff) == 0x81} { puts hit }
```

**Parenthesise as parsed** keeps the current behaviour and makes it explicit.
Hover an operator to see the expression's parse as a tree.

## How to suppress

Add the parentheses either way, use `# tcl-lsp: disable=W154`, or disable
W154 in editor diagnostic settings.

## Related

- [KCS codes index](README.md)
- [W155 — non-associative grouping](kcs-diagnostic-w155-non-associative-grouping.md)
- [IRULE5008 — signed scan value compared with an unsigned constant](kcs-diagnostic-irule5008-signed-scan-comparison.md)
- [Execution evidence: operator groupings](../../../evidence/tcl/expr-precedence-groupings.md)
- [Execution evidence: `&` versus `==`](../../../evidence/tcl/binary-scan-signed-byte-comparison.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)

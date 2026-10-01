# KCS: W110 — Why should I use eq/ne instead of ==/!= for strings?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about using `==` or `!=` to compare strings?

## Why

The `==` and `!=` operators compare numerically when both operands are numbers, and as strings otherwise. When one operand is a fixed string that is not a number, such as `"admin"`, the comparison is already a string comparison. `eq` and `ne` say so explicitly, and give exactly the same answer.

W110 fires only in that case, where the rewrite is proven to keep the result. The comparison must be in a braced expression, and one operand must be a fixed string that is not a number in any Tcl release. It stays quiet on these, because `eq` would change the answer:

- `$x == "42"` is true for `x` = `42.0`, and `$x eq "42"` is false. The same goes for any number-shaped string, including ` 1`, `nan`, `0x10`, `08` (a number from Tcl 9.0) and `1_0` (9.0).
- `"$x" == 1` substitutes `x` first, so it is a numeric comparison when `x` is `1.0`.
- An unbraced condition such as `if "\$x == {a}"` is substituted before `expr` sees it, so its operands are not known.

A boolean word is a string here: `==` never reads `true` as `1`, so `$x == "true"` is a string comparison and W110 fires.

## Symptoms

- A hint underline under the operator, with the message "Use 'eq' instead of
  '==' for string comparison in expressions to avoid ambiguous numeric/string
  coercion."
- A **Use 'eq' for string comparison** quick fix. It rewrites only the
  proven operators, in place, and leaves every other byte of the expression
  as written. It is classed as semantics-equivalent, so "Fix All Safe
  Issues" applies it.

## Example that triggers it

```tcl
set name [gets stdin]
if {$name == "admin"} { puts "welcome" }
```

The analyser reports **`W110`** on the `==` operator.

## Fix

```tcl
set name [gets stdin]
if {$name eq "admin"} { puts "welcome" }
```

Use `eq` for equality and `ne` for inequality when comparing strings.

## How to suppress

Add `# noqa: W110` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W114`

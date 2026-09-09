# KCS: W114 — Why is a nested [expr] inside an expression redundant?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about a nested `[expr]` inside an expression context?

## Why

Commands like `if`, `while`, and `expr` already evaluate their argument as an expression. Nesting another `[expr]` adds overhead, prevents optimisation, and gains nothing.

## Symptoms

- A yellow squiggle under the inner `[expr ...]`, with the message "Redundant
  nested [expr] — already in expression context".
- An **Unwrap the nested `expr`** quick fix when the outer expression and the
  inner body are both braced.

## Example that triggers it

```tcl
set a [gets stdin]
if {[expr {$a + 1}] > 10} { puts "big" }
```

The analyser reports **`W114`** on the inner `[expr {$a + 1}]`.

## Fix

```tcl
set a [gets stdin]
if {($a + 1) > 10} { puts "big" }
```

Inline the sub-expression directly; the outer context already evaluates it.

The quick fix replaces the nested `[expr {...}]` with its body in parentheses,
or with the bare body when that is a single number or a lone `$var`. It is
offered only when the inline is purely textual: the outer expression braced,
the inner body one braced group, and no string comparison (`eq`, `ne`, `in`,
`ni`) in the outer expression — a nested `expr` normalises a numeric result, so
unwrapping could flip a string comparison. An unbraced inner body stays
message-only, because inlining it would expose the text to another round of
substitution.

## How to suppress

Add `# noqa: W114` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W110`

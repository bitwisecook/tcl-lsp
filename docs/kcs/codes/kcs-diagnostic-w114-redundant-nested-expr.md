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

- A yellow squiggle appears under the inner `[expr ...]`, with the message "redundant nested [expr] inside expression context".
- When the outer expression and the inner body are both braced, a quick fix titled "Unwrap the nested `expr`" is offered.

## Example that triggers it

```tcl
if {[expr {$a + 1}] > 10} { puts "big" }
```

The analyser reports **`W114`** on the inner `[expr {$a + 1}]`.

## Fix

```tcl
if {($a + 1) > 10} { puts "big" }
```

Inline the sub-expression directly; the outer context already evaluates it.

The quick fix replaces the nested `[expr {...}]` with its body in parentheses — or with the bare body when it is a single number or a lone `$var`, where parentheses cannot change parsing. It is offered only when the inline is purely textual: the outer expression is braced, the inner body is one braced group, and the outer expression contains no string comparison (`eq`, `ne`, `in`, or `ni` — a nested `expr` normalises a numeric result, so unwrapping could flip a string comparison's verdict). An unbraced inner body stays message-only, because inlining it would expose the text to another round of substitution.

## How to suppress

Add `# noqa: W114` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W110`

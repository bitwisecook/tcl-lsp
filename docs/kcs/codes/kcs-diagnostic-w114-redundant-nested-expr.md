# KCS: W114 — Why is a nested [expr] inside an expression redundant?

> **Audience:** User
> **Type:** Issue

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

## How to suppress

Add `# noqa: W114` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W110`

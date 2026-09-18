# KCS: O122 — Convert tail-recursive proc to iterative while loop

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, tail-call

## Profiles

full

## Question

What does O122 rewrite, and when does it fire?

## Why

An iterative loop has zero call overhead and cannot overflow the stack regardless of input size. This fires when a fully tail-recursive proc can be expressed as a `while` loop.

## Before

```tcl
proc fact {n} {
    if {$n <= 1} { return 1 } else { fact [expr {$n - 1}] }
}
```

## After

```tcl
proc fact {n} {
    while {1} {
        if {$n <= 1} { return 1 } else { set n [expr {$n - 1}] }
    }
}
```

The recursive call becomes a reassignment of the parameters, and the body is
wrapped in `while {1}`; the existing `return`s are what leave the loop. A proc
with several parameters reassigns them together with `lassign`.

## Safety conditions

- Skipped when any self-call is **not** in [tail position](../../GLOSSARY.md#tail-position), including one in an `if` / `while` / `for` condition or a `switch` subject — the loop body would still evaluate that call recursively.
- Skipped when a recursive call passes a different number of arguments than the proc declares. A bracketed argument such as `[expr {$n - 1}]` is one argument.
- Skipped when a recursive call expands its arguments with `{*}`, whose word count is not known until runtime.
- Skipped on Tcl 8.4 for a proc with more than one parameter, which has no `lassign` to reassign them.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Tail-call analysis](../../GLOSSARY.md#tail-call-optimisation)
- Related codes: `O121`, `O123`

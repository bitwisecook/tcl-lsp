# KCS: O130 — Fold static lappend list build chains into a single assignment

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, pattern

## Profiles

standard, full

## Question

What does O130 rewrite, and when does it fire?

## Why

A common Tcl idiom initialises a list with `set l {}` and then populates it
with one or more `lappend` calls before the list is first read. When all the
appended elements are compile-time constants and the variable is not read or
observed between the writes, the entire sequence can be collapsed into one
`set` statement with the final list literal. This removes the intermediate
allocations and `lappend` calls at runtime.

## Before

```tcl
set l {}
lappend l a b
puts $l
```

## After

```tcl
set l {a b}
puts $l
```

## Safety conditions

- Every element appended by `lappend` must be a compile-time constant — no
  variable references or command substitutions.
- The variable must not be read between the initial `set` and the final
  `lappend` in the chain; an intervening read would observe the partial list
  and must not be dropped.
- Skipped when the variable is aliased to an outer scope via `global`,
  `variable`, or `upvar`, or is under a `trace` at any write in the chain —
  those writes are observable and cannot be collapsed.
- Only applies to write-only list build chains; mixed `append`/`lappend`
  chains are handled by `O104`.

## How to disable

Toggle the optimiser profile in your editor settings. See the
[optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [pattern recognition](../../GLOSSARY.md#pattern-recognition)
- Related codes: `O104`, `O116`, `O119`

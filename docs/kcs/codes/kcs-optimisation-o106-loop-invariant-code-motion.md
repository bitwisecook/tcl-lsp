# KCS: O106 — Hoist loop-invariant computations

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, licm

## Profiles

full

## Question

What does O106 rewrite, and when does it fire?

## Why

A computation that produces the same value on every iteration runs once instead of N times when hoisted above the loop.

## Before

```tcl
foreach i $list {
    set n [llength $list]
    # ...
}
```

## After

```tcl
set n [llength $list]
foreach i $list {
    # ...
}
```

O106 reports the invariant computation and leaves the source as written; the
hoist above is the edit you make.

## Safety conditions

- Skipped when the hoisted expression depends on a variable modified inside the loop body.
- Skipped when the expression has side effects.
- Skipped inside a procedure, method, or `apply` body. Hoisting changes how
  many times a command is dispatched, and a caller can install an execution
  trace before the body runs, so only a loop at the top level of a file can
  prove the dispatch is stable.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [LICM](../../GLOSSARY.md#licm)
- Related codes: `O105`, `O110`

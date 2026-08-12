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

## Safety conditions

- Skipped when the hoisted expression depends on a variable modified inside the loop body.
- Skipped when the expression has side effects.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [LICM](../../GLOSSARY.md#licm)
- Related codes: `O105`, `O110`

# KCS: O119 — Pack consecutive set literals into lassign

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, pattern

## Profiles

standard, full

## Question

What does O119 rewrite, and when does it fire?

## Why

Multiple `set` statements for related assignments can be expressed as a single `lassign`, reducing instruction count and making the grouping explicit.

## Before

```tcl
set a 1
set b 2
set c 3
```

## After

```tcl
lassign {1 2 3} a b c
```

## Safety conditions

- Skipped when any of the assigned values is not a compile-time constant.
- Skipped when the variables have [traces](../../GLOSSARY.md#trace) that depend on being set individually.
- Skipped when a later assignment reads a variable set earlier in the same group.
- Skipped on Tcl 9.0, where individual `set`s are faster. Tcl 8.5 and 8.6 get
  `lassign`; Tcl 8.4 gets `foreach {a b c} {1 2 3} {break}`, which every
  version understands.
- Only a strictly consecutive run is packed — the optimiser does not reorder
  statements to bring candidates together.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Pattern recognition](../../GLOSSARY.md#pattern-recognition)
- Related codes: `O100`, `O116`

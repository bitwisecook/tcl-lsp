# KCS: O117 — Simplify string length check to empty-string comparison

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

readability, standard, full

## Question

What does O117 rewrite, and when does it fire?

## Why

An empty-string comparison is cheaper than computing the length and comparing it to zero. The `eq ""` form also reads more naturally and avoids a function call.

## Before

```tcl
if {[string length $s] == 0} { ... }
```

## After

```tcl
if {$s eq ""} { ... }
```

## Safety conditions

- Skipped when the comparison operator is not `==` or `!=` against zero.
- Skipped when the variable may hold a value whose [shimmer](../../GLOSSARY.md#shimmer) from string to integer would be observable.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O115`, `O120`

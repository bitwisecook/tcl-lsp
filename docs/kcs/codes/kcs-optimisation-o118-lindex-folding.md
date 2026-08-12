# KCS: O118 — Fold constant lindex to element

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O118 rewrite, and when does it fire?

## Why

When both the list and the index are known at compile time, the element can be resolved statically. This removes a runtime call to `lindex` and produces a simple constant.

## Before

```tcl
set val [lindex {red green blue} 1]
```

## After

```tcl
set val green
```

## Safety conditions

- Skipped when the list or index is not a compile-time constant.
- Skipped when the index is out of range.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O101`, `O116`

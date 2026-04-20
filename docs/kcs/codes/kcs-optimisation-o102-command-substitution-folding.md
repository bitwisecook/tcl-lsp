# KCS: O102 — Fold constant command substitutions

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O102 rewrite, and when does it fire?

## Why

A command substitution whose inputs are all constants can be replaced with the result, eliminating the call at runtime.

## Before

```tcl
set x [expr {2 * 3}]
```

## After

```tcl
set x 6
```

## Safety conditions

- Skipped when the substituted command has side effects.
- Skipped when any argument to the inner command is not a compile-time constant.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O101`

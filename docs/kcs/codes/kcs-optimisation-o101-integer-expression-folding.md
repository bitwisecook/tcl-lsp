# KCS: O101 — Fold constant integer expressions

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O101 rewrite, and when does it fire?

## Why

Evaluating arithmetic at compile time avoids runtime computation, producing smaller, faster code.

## Before

```tcl
expr {2 + 3}
```

## After

```tcl
5
```

## Safety conditions

- Skipped when any operand is not a compile-time constant.
- Skipped when the expression involves floating-point division that could lose precision.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O102`

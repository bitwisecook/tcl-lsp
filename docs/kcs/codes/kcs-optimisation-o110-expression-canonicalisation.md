# KCS: O110 — Canonicalise expressions (InstCombine)

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, instcombine

## Profiles

standard, full

## Question

What does O110 rewrite, and when does it fire?

## Why

Normalising equivalent expressions reveals redundancies and enables further folding by later passes.

## Before

```tcl
expr {$x * 1}
```

## After

```tcl
expr {$x}
```

## Safety conditions

- Skipped when the rewrite would change the result type (e.g. integer vs. floating-point).
- Skipped when the original expression has observable side effects that the simplified form would drop.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [InstCombine](../../GLOSSARY.md#instcombine)
- Related codes: `O100`, `O101`

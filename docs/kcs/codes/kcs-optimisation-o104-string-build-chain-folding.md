# KCS: O104 — Fold static string build chains

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O104 rewrite, and when does it fire?

## Why

Consecutive append or concat operations on constants can be collapsed into one assignment, reducing runtime string allocations.

## Before

```tcl
set s "hello"
append s " world"
```

## After

```tcl
set s "hello world"
```

## Safety conditions

- Skipped when any value in the chain is not a compile-time constant.
- Skipped when the variable is read between the initial assignment and the final append.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O105`

# KCS: O100 — Propagate constant variables into expressions

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O100 rewrite, and when does it fire?

## Why

Replacing variable references with known constants lets later passes fold the expression entirely, removing runtime look-ups.

## Before

```tcl
set n 10
expr {$n + 1}
```

## After

```tcl
set n 10
expr {10 + 1}
```

## Safety conditions

- Skipped when the variable may be modified between assignment and use (e.g. by `upvar`, `trace`, or an intervening command call).
- Skipped when the constant value contains metacharacters that would change meaning in the target context.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O101`, `O105`

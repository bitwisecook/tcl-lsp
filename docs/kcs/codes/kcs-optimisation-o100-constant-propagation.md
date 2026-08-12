# KCS: O100 — Propagate constant variables into expressions

> **Audience:** User
> **Type:** Optimisation

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

- Skipped when the variable is aliased (`global`, `variable`, `upvar`) or traced anywhere in its own procedure, or — for a top-level variable specifically — when *any* procedure in the file reassigns it via `global`. A top-level name already lives in the global frame, so a procedure elsewhere can rewrite it between the assignment and a later top-level use even though the top-level code itself never mentions `global`.
- Skipped when the constant value contains metacharacters that would change meaning in the target context.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O101`, `O105`

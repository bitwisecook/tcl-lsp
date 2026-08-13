# KCS: O115 — Remove redundant nested expr

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

readability, standard, full

## Question

What does O115 rewrite, and when does it fire?

## Why

The inner `[expr]` is already evaluated in an expression context; the nesting adds overhead for no benefit. Removing it simplifies the code and avoids a redundant parse-and-evaluate cycle.

## Before

```tcl
if {[expr {$x > 0}]} { ... }
```

## After

```tcl
if {$x > 0} { ... }
```

## Safety conditions

- Skipped when the inner expression contains side effects (e.g. command substitutions) whose evaluation order would change.
- Skipped when the inner `[expr]` is unbraced, as removing it could alter substitution semantics.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O101`, `O114`, `O117`

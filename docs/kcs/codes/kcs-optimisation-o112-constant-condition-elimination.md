# KCS: O112 — Eliminate constant-condition compound statements

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O112 rewrite, and when does it fire?

## Why

An `if`/`while` whose condition is always true or always false can be collapsed to the live branch or removed entirely, simplifying the control flow.

## Before

```tcl
if {1} { puts yes } else { puts no }
```

## After

```tcl
puts yes
```

## Safety conditions

- Skipped when the condition is not a compile-time constant.
- Skipped when the dead branch contains side effects that must be preserved for correctness.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `O107`, `O108`

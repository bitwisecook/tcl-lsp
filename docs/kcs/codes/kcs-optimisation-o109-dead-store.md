# KCS: O109 — Eliminate dead stores

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O109 rewrite, and when does it fire?

## Why

A variable assigned but overwritten before any read wastes the computation; removing the dead store simplifies the code.

## Before

```tcl
set limit 1
set limit 2
puts $limit
```

## After

```tcl
set limit 2
puts 2
```

O109 drops the first `set`. The surviving literal is then forwarded into the
read by [O102](kcs-optimisation-o102-load-forwarding.md).

## Safety conditions

- Skipped when the right-hand side of the dead store has side effects.
- Skipped when the variable could be observed externally (e.g. via `upvar` or `trace`).

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `O107`, `O108`

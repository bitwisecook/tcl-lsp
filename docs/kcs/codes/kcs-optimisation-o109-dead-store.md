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
- Skipped when evaluating the right-hand side could raise an error, because deleting the statement would drop the error. A value that reads a variable is kept unless every variable it reads is definitely set here (a parameter, or assigned on every path), and an `expr` or `incr` value is kept unless it folds to a constant. So `set y $x` with `x` unset, `set y [expr {$v + 1}]` with `v` a parameter, and `set y [expr {1/0}]` all stay.
- Skipped when the variable could be observed externally (e.g. via `upvar` or `trace`).

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `O107`, `O108`

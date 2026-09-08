# KCS: O114 — Recognise incr idiom

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, pattern

## Profiles

readability, standard, full

## Question

What does O114 rewrite, and when does it fire?

## Why

`incr` is a single bytecode instruction; the `expr` form requires parsing and evaluation. Rewriting `set x [expr {$x + N}]` to `incr x N` produces shorter, faster, and more idiomatic code.

## Before

```tcl
set count 0
set count [expr {$count + 1}]
```

## After

```tcl
set count 0
incr count
```

## Safety conditions

- Skipped when the increment value is not an integer constant.
- Skipped when the variable has no reaching definition the optimiser can see — a procedure parameter, or a value read from a call.
- Skipped when the variable is subject to a [trace](../../GLOSSARY.md#trace) that could observe the difference between `set` and `incr`.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Pattern recognition](../../GLOSSARY.md#pattern-recognition)
- Related codes: `O100`, `O113`, `O120`

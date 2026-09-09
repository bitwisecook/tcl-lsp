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
set area [expr {$side ** 2}]
```

## After

```tcl
set area [expr {$side * $side}]
```

O110 rewrites the right-hand side of a `set name [expr {…}]` and an `if` /
`while` condition. When the reducible operation is the *whole* branch
condition, the same rewrite is reported as
[O113](kcs-optimisation-o113-strength-reduction.md) instead — that cascade
tries strength reduction first.

## Safety conditions

- Skipped when the rewrite would change the result type (e.g. integer vs. floating-point). `$x * 1`, `$x + 0`, and `$x * 0` therefore need an operand proved to be a number.
- Skipped when the original expression has observable side effects that the simplified form would drop.
- Skipped when an operand could be `NaN` and the rewrite depends on it not
  being. With a `NaN` operand Tcl makes `!=` true and every other comparison
  false, so `!($x < $y)` is not `$x >= $y`, and `$x == $x` is `0` rather than
  `1`. Rewrites of `<`, `<=`, `>`, `>=` under a `!`, and the `$x == $x` /
  `$x != $x` / `$x <= $x` / `$x >= $x` folds, therefore fire only where the
  operand is proved to be an integer. `==` / `!=` inversions, `$x < $x`,
  `$x > $x`, and every string comparison (`eq`, `ne`, `lt`, …) and membership
  test (`in`, `ni`) are unaffected.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [InstCombine](../../GLOSSARY.md#instcombine)
- Related codes: `O100`, `O101`

# KCS: O113 — Strength-reduce expensive expressions

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, strength-reduce

## Profiles

standard, full

## Question

What does O113 rewrite, and when does it fire?

## Why

Replacing expensive operations with cheaper equivalents reduces CPU cost per evaluation. Exponentiation and modulo by powers of two are common targets because multiplication and bitwise AND produce the same result for significantly less work.

## Before

```tcl
expr {$x ** 2}
```

## After

```tcl
expr {$x * $x}
```

## Safety conditions

- Skipped when the exponent is not a small compile-time constant.
- Skipped for modulo reduction (`x%N` to `x&(N-1)`) when the divisor is not a power of two or the operand may be negative.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Strength reduction](../../GLOSSARY.md#strength-reduction)
- Related codes: `O100`, `O101`

# KCS: O120 — Prefer eq/ne for string comparisons

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

readability, standard, full

## Question

What does O120 rewrite, and when does it fire?

## Why

`==` and `!=` attempt numeric coercion first; `eq` and `ne` compare strings directly, which is faster and avoids surprising type conversion. When one operand is a string literal, string comparison is the correct intent.

## Before

```tcl
if {$name == "admin"} { ... }
```

## After

```tcl
if {$name eq "admin"} { ... }
```

## Safety conditions

- Skipped when both operands could be numeric and the comparison is intentionally arithmetic.
- Skipped when the operand types cannot be determined statically.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O114`, `O117`

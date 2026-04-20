# KCS: O103 — Fold static procedure calls

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, ipa

## Profiles

standard, full

## Question

What does O103 rewrite, and when does it fire?

## Why

When all arguments to a pure proc are constant, the call can be replaced with its return value, removing the function-call overhead entirely.

## Before

```tcl
proc double {n} { expr {$n * 2} }
set x [double 21]
```

## After

```tcl
set x 42
```

## Safety conditions

- Skipped when the proc has observable side effects.
- Skipped when any argument is not a compile-time constant.
- Skipped when the proc body cannot be summarised by [interprocedural analysis](../../GLOSSARY.md#ipa).

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [IPA](../../GLOSSARY.md#ipa)
- Related codes: `O100`, `O102`

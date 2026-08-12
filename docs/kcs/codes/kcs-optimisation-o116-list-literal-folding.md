# KCS: O116 — Fold constant list to literal

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O116 rewrite, and when does it fire?

## Why

A list built from constants can be replaced with the literal result, avoiding runtime construction. The `[list]` call is redundant when every element is a plain string with no special characters.

## Before

```tcl
set items [list a b c]
```

## After

```tcl
set items {a b c}
```

## Safety conditions

- Skipped when any element contains whitespace, braces, or backslashes that would require quoting in the literal form.
- Skipped when elements are not compile-time constants.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O101`, `O118`

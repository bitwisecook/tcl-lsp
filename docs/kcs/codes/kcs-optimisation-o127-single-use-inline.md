# KCS: O127 — Inline single-use variable assignment

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O127 rewrite, and when does it fire?

## Why

A variable read exactly once can be replaced with its value, removing a needless temporary. This shortens the code and avoids an extra variable look-up at runtime.

## Before

```tcl
set uri [HTTP::uri]
pool_select $uri
```

## After

```tcl
pool_select [HTTP::uri]
```

## Safety conditions

- Skipped when the right-hand side has [side effects](../../GLOSSARY.md#side-effects) and the inline position would change evaluation order.
- Skipped when the variable is read more than once.
- Skipped when a [barrier](../../GLOSSARY.md#barrier) between the assignment and the use could alter the value.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Dead-code elimination](../../GLOSSARY.md#dce)
- Related codes: `O125`, `O126`

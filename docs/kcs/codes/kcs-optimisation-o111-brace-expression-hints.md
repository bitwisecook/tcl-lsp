# KCS: O111 — Brace expression performance hints

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

readability, standard, full

## Question

What does O111 rewrite, and when does it fire?

## Why

Braced expressions compile to bytecode; unbraced ones are re-parsed on every call, which is slower and risks double substitution.

## Before

```tcl
expr $x + 1
```

## After

```tcl
expr {$x + 1}
```

## Safety conditions

- Skipped when the unbraced form relies on double substitution intentionally.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O101`

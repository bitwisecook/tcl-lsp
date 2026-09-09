# KCS: O127 — Inline single-use variable assignment

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O127 rewrite, and when does it fire?

## Why

A variable read exactly once does not need a statement of its own. Folding the
`set` into the use site drops the separate statement and the variable look-up
that followed it.

## Before

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    log local0. $uri
}
```

## After

```tcl
when HTTP_REQUEST {
    log local0. [set uri [HTTP::uri]]
}
```

The `set` moves into the argument rather than disappearing: it still returns
the value it assigns, so the variable is left in place for anything that reads
it later through a dynamic name.

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

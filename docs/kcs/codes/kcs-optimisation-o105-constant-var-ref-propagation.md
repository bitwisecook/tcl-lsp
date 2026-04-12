# KCS: O105 — Propagate constants into variable references (GVN/CSE)

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, gvn

## Profiles

standard, full

## Question

What does O105 rewrite, and when does it fire?

## Why

Replacing `$var` with its known value and eliminating duplicate computations reduces runtime work and memory traffic.

## Before

```tcl
set uri [HTTP::uri]
set a $uri
set b [HTTP::uri]
```

## After

```tcl
set uri [HTTP::uri]
set a $uri
set b $uri
```

## Safety conditions

- Skipped when the variable may be modified between definition and use.
- Skipped when the duplicated command has side effects that must execute twice.
- Skipped when the value contains metacharacters unsafe for the target context.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [GVN](../../GLOSSARY.md#gvn)
- Related codes: `O100`, `O106`

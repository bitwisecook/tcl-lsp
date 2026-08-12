# KCS: O124 — Comment out unused procs in iRules

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, unused-procs

## Profiles

full

## Question

What does O124 rewrite, and when does it fire?

## Why

Procs that are defined but never called from any event waste parse time and confuse readers. Commenting them out keeps the definition available for reference while removing it from the active code path.

## Before

```tcl
proc legacy {} { ... }
when HTTP_REQUEST { pool main }
```

## After

```tcl
# proc legacy {} { ... }
when HTTP_REQUEST { pool main }
```

## Safety conditions

- Skipped when the proc is called from any `when` event block, directly or indirectly.
- Skipped when the proc name matches a pattern registered as externally callable.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Unused-procs analysis](../../GLOSSARY.md#unused-procs-elimination)
- Related codes: `O126`, `O127`

# KCS: O126 — Remove set statements for variables never read

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O126 rewrite, and when does it fire?

## Why

A variable that is set but never read is dead code; removing it simplifies the script and eliminates a wasted assignment.

## Before

```tcl
set unused 42
puts "done"
```

## After

```tcl
puts "done"
```

## Safety conditions

- Skipped when the `set` command's right-hand side has [side effects](../../GLOSSARY.md#side-effects) that must be preserved.
- Skipped when the variable has a [trace](../../GLOSSARY.md#trace) attached.
- Skipped when the variable could be read via `upvar`, `uplevel`, or other dynamic access.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Dead-code elimination](../../GLOSSARY.md#dce)
- Related codes: `O124`, `O125`, `O127`

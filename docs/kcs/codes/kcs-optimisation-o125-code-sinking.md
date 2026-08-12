# KCS: O125 — Sink assignments into the deepest decision block

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, code-sinking

## Profiles

full

## Question

What does O125 rewrite, and when does it fire?

## Why

Moving an assignment into the branch that uses it means the hot path skips work it does not need. This reduces runtime cost when the variable is only consumed in a cold branch.

## Before

```tcl
set msg "error"
if {$ok} { return } else { log $msg }
```

## After

```tcl
if {$ok} { return } else { set msg "error"; log $msg }
```

## Safety conditions

- Skipped when the assignment has [side effects](../../GLOSSARY.md#side-effects) that must execute unconditionally.
- Skipped when the variable is read in more than one branch.
- Skipped when a [barrier](../../GLOSSARY.md#barrier) between the original position and the sink target could observe the variable.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Code sinking](../../GLOSSARY.md#lcp)
- Related codes: `O126`, `O127`

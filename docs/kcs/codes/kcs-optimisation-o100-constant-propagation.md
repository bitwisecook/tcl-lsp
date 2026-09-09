# KCS: O100 — Propagate constant variables into expressions

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O100 rewrite, and when does it fire?

## Why

Replacing variable references with known constants lets later passes fold the expression entirely, removing runtime look-ups.

## Before

```tcl
set retries 1
incr retries
puts "$retries"
```

## After

```tcl
set retries 1
incr retries
puts 2
```

`incr` is not a load of `retries` — it names the cell it mutates — so there
is no literal for [O102](kcs-optimisation-o102-load-forwarding.md) to
forward. Constant propagation has nonetheless proved this read sees `2`, so
O100 inlines it. The now-dead `set` and `incr` go to
[O108](kcs-optimisation-o108-transitive-dead-code.md) /
[O109](kcs-optimisation-o109-dead-store.md) on the next pass, so the
`aggressive` profile's fixpoint reduces the snippet to `puts 2`.

## The other shapes O100 rewrites

The same proven constant is substituted into a `return $var`, into the
right-hand side of `set x [expr {…}]`, and into an `if` / `while` condition.
Forwarding a variable whose single reaching definition is *written out* as a
literal is a different rewrite —
[O102](kcs-optimisation-o102-load-forwarding.md). O100 covers the reads
whose value had to be proved rather than read off the source.

## Safety conditions

- Skipped when the variable is aliased (`global`, `variable`, `upvar`) or traced anywhere in its own procedure, or — for a top-level variable specifically — when *any* procedure in the file reassigns it via `global`. A top-level name already lives in the global frame, so a procedure elsewhere can rewrite it between the assignment and a later top-level use even though the top-level code itself never mentions `global`.
- Skipped when the constant value contains metacharacters that would change meaning in the target context.
- Skipped when the read has more than one reaching definition, or a barrier sits between the definition and the read.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O101`, `O102`, `O105`, `O108`, `O109`

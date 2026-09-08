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
set n 10
expr {$n + 1}
```

## After

```tcl
set n 10
expr {10 + 1}
```

## Also: a constant proved at one particular read

Most of O100 substitutes by *name*: it asks "is this variable always this
constant", which is the only question a source rewrite keyed on a name can
ask. That answer is unavailable for any variable reassigned to a different
constant — a counter, most obviously.

Where the read is reached through the def-use chains, O100 can ask the
stronger, per-value question instead: "what did SCCP prove *this* read
sees". That covers a definition which computes its value rather than
writing one out:

```tcl
set a 1
incr a
puts "$a"
```

`incr` is not a load of `a` — it names the cell it mutates — so there is no
literal for [O102](kcs-optimisation-o102-load-forwarding.md) to forward.
SCCP has nonetheless proved the read sees `2`, so O100 inlines it, giving
`puts 2`; the now-dead `set` and `incr` go to `O108`/`O109` on the next
pass, and the `aggressive` profile's fixpoint reduces the whole snippet to
`puts 2`.

The same guards apply as for the by-name form, plus every guard O102's
def-use walk applies (single reaching definition, no trace, no alias, no
intervening barrier).

## Safety conditions

- Skipped when the variable is aliased (`global`, `variable`, `upvar`) or traced anywhere in its own procedure, or — for a top-level variable specifically — when *any* procedure in the file reassigns it via `global`. A top-level name already lives in the global frame, so a procedure elsewhere can rewrite it between the assignment and a later top-level use even though the top-level code itself never mentions `global`.
- Skipped when the constant value contains metacharacters that would change meaning in the target context.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O101`, `O102`, `O105`, `O108`, `O109`

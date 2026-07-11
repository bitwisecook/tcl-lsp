# KCS: O102 — Forward a variable's single reaching literal load

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O102 rewrite, and when does it fire?

## Why

When a variable has exactly one reaching definition and that
definition is a literal value, reading the variable is equivalent to
writing the literal directly — the read costs a variable lookup for
no benefit. Forwarding the literal removes that lookup and often
exposes further folding at the use site (a nested `[expr {...}]}`
substitution that becomes foldable once its operand is a literal, for
example — see [O101](kcs-optimisation-o101-integer-expression-folding.md)
and [O129](kcs-optimisation-o129-builtin-command-substitution-folding.md)
for the folds this commonly feeds).

## Before

```tcl
set n 7
puts $n
```

## After

```tcl
puts 7
```

## Safety conditions

O102 only forwards when the compiler can prove the substitution is
value-identical to the original read, in every one of these senses:

- The variable has a **single reaching definition** on every path
  to the read, and that definition is a literal (not a computed
  value).
- The variable carries **no active variable trace**
  (`trace add variable`/`trace variable`, or the `remove`/`vdelete`
  spellings) anywhere in the module, and no dynamic
  (`trace add variable $name ...`) trace target — a trace handler can
  run arbitrary code on read, and a write-trace handler can rewrite
  the value actually stored, so neither the literal text nor the
  absence of an observable read can be trusted once a trace is
  possible.
- The variable is not **aliased** into another stack frame
  (`upvar`/`global`/`variable`) — a proc reached between the
  definition and the read could write it through the alias.
- No statement between the definition and the read (within the same
  block) is a **barrier** (`eval`/`uplevel`/`interp eval`/…) or a call
  the compiler cannot prove pure — including any call to a
  user-defined proc, since an unrecognised command is treated
  conservatively as an unproven write.
- The taint above propagates **transitively**: a variable whose
  value is computed from an unsafe read (`set v [expr {$a * 2}]}`
  where `a` is traced) is unsafe too, even though `v` itself carries
  no trace.

Where these conditions hold and the use is a bare `$var` / `${var}`
word in a command argument, O102 emits a **precise, applicable** fix
targeting just that word. Where the use is nested inside a larger
construct the compiler cannot yet target precisely (a variable
reference inside a `"..."` interpolated string is handled by the
related [O100](kcs-optimisation-o100-constant-propagation.md) path
instead; a reference nested inside an arbitrary command substitution
falls back to a **hint-only** suggestion covering the whole
statement, with no automatic fix offered).

## Related history

Earlier documentation for this code described it as "fold constant
`[expr {...}]}` command substitutions" — a description carried over
from an earlier implementation. The Rust optimiser's O102 is the more
general load-forwarding rewrite described above; folding a
pure-literal `[expr {...}]}` substitution with no propagated variable
involved is [O101](kcs-optimisation-o101-integer-expression-folding.md)'s
job. The two commonly co-fire: propagating a literal into an
`[expr {...}]}` operand (O102) frequently exposes an O101 fold of the
resulting expression.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O101`, `O127`

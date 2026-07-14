# KCS: S103 — Why does the analyser warn about mutating a shared value?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser flag `lappend`, `lset`, or a `dict` mutator on a variable that was assigned from another variable?

## Why

Tcl never mutates a value another variable can still see: every list/dict
mutator duplicates a shared value before writing (`Tcl_IsShared` →
`Tcl_DuplicateObj` in C Tcl). `set b $a` does not copy anything — it makes
both variables hold the *same* object — so the first `lappend b` afterwards
silently copies the entire value, an O(n) cost that repeats every time the
pattern runs.

## Symptoms

- A hint underline appears under the mutating command, with the message
  "mutation of a potentially shared value copies it".

## Example that triggers it

```tcl
set a [lrepeat 1000 x]
set b $a          ;# a and b now share one 1000-element list
lappend b y       ;# duplicates all 1000 elements before appending
puts [llength $a] ;# a still holds (and reads) the original
```

The analyser reports **`S103`** because `b`'s value is still shared with
`a` when `lappend` writes to it, so Tcl copies the whole list first.
Mutating `a` instead (while `b` is still read later) is flagged the same
way — the duplication is symmetric.

## When it does not fire

- **Mutating a proc parameter directly** (`proc p {l} { lappend l x }`) —
  procedures receive shared argument objects by design, and the first-write
  copy is idiomatic Tcl.
- **Explicit copies** (`set b [lrange $a 0 end]`) — the analyser only pairs
  plain `set b $a` copy assignments.
- **A source that is dead or reassigned** before/after the mutation
  (`set b $a; set a other; lappend b x` mutates a sole owner — no copy).
- **Array elements, `upvar`/`global`/`variable` aliases, traced
  variables** — another route can reach the value, so the analyser
  abstains.

## Fix

Avoid holding two live variables over the value you mutate — mutate the
original, drop the alias first, or build the variant directly:

```tcl
set a [lrepeat 1000 x]
lappend a y                 ;# mutate the sole owner in place
```

or, when a distinct copy is genuinely wanted, accept the one-off copy but
stop re-reading the stale alias so later mutations run in place.

## How to suppress

Add `# noqa: S103` on the line **above** the offending command, or apply
the "Suppress S103 with a noqa comment" quick fix offered on the
diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S100`, `S101`, `S102`

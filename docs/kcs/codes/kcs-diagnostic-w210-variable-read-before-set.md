# KCS: W210 — Why does the analyser warn about a variable used before being set?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, liveness

## Profiles

default

## Question

Why does the analyser flag a variable that is read before it has been assigned a value?

## Why

Reading an undefined variable causes a runtime error and stops the script.

## Symptoms

- A yellow squiggle appears under the variable reference, with the message "variable used before being set".

## Example that triggers it

```tcl
puts $x
```

The analyser reports **`W210`** because `x` is never set before it is read.

## Fix

```tcl
set x ""
puts $x
```

Assign the variable before using it.

## Existence checks are not reads

`[info exists X]` tests whether `X` exists, and `[array exists X]` tests whether
`X` exists *as an array variable* (a scalar `set X 1` makes `info exists` true
but leaves `array exists` false). Both *test* rather than read the value, so
neither raises `W210`. A check also informs the branches it guards: inside
`if {[info exists X]} { … }` the variable is known to exist, so reading `$X`
there is safe; on the `else` side it is still unset, so a read there is still
flagged. When existence is statically provable, the check is folded to a
constant and reported as
[`I230`](kcs-diagnostic-i230-constant-existence-check.md) instead — though a
scalar assignment only proves `info exists`, never `array exists`.

The same branch narrowing applies to the `info vars` / `info locals` membership
idioms for a single exact name: `[info vars X] ne ""`,
`[llength [info vars X]]`, and `[lsearch [info vars] X] > -1`; and to
`catch {set _ $X}`, whose no-error (false) branch proves `$X` was readable.
(`info globals` is not used — it proves the *global* exists, not the bare-`$X`
local — and glob patterns are not statically decidable.)

## How to suppress

Add `# noqa: W210` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W211`, `W213`, `W220`

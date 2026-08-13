# KCS: W231 — Why does the analyser warn about an `lset` index that is out of range?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, const-fold

## Profiles

default

## Question

Why does the analyser flag a constant index passed to `lset` that falls
outside the list?

## Why

Unlike `lindex` and friends, `lset` **raises an "index ... out of
range" error** at runtime when the index is invalid. A negative literal
is always invalid; an `end-N` that underflows the list length is also
invalid.

The analyser resolves the list length in two ways:

1. If the preceding `set var {...}` used a literal list, the analyser
   uses its element count.
2. Otherwise, any plain negative integer literal is still flagged
   (always invalid on `lset`).

## Example that triggers it

```tcl
set xs {a b c}
lset xs -1 X       ;# negative literal -> runtime error
lset xs 5 X        ;# list has 3 elements -> runtime error

set ys {only}
lset ys end-2 X    ;# end = 0, end-2 = -2 -> runtime error
```

The analyser reports **`W231`** and quotes the resolved index.

## Fix

Use a valid index inside `[0, llength $xs)`. If the list may be shorter
than expected, check `llength $xs` first:

```tcl
if {[llength $xs] > 5} {
    lset xs 5 X
}
```

## How to suppress

Add `# noqa: W231` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W230 — list index silently out of range](kcs-diagnostic-w230-list-index-out-of-range.md)
- [W232 — string index out of range](kcs-diagnostic-w232-string-index-out-of-range.md)

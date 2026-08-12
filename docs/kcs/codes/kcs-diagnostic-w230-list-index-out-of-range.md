# KCS: W230 — Why does the analyser warn about a list index that is out of range?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, const-fold

## Profiles

default

## Question

Why does the analyser flag a constant index passed to `lindex`,
`lrange`, or `lreplace` that falls outside the list?

## Why

In Tcl 9, these commands silently return the empty string, clamp the
range, or (for `lreplace`) prepend/append instead of replacing when the
index is out of bounds. That silent behaviour hides real bugs: the
programmer usually expected an element and will never see the error.
`linsert` is deliberately excluded — its clamp always produces a
sensible result, so flagging it would second-guess intent.

The analyser checks two constant shapes:

- A plain integer like `-1` or `5`.
- An `end-N` expression where `N` is larger than the list length minus
  one (so the resolved offset is negative).

## Example that triggers it

```tcl
set xs {a b c}
set first [lindex $xs -1]   ;# want end, got ""
set tail  [lindex $xs end-5] ;# list only has 3 elements -> ""
set slice [lrange {a b c} 10 20]
```

The analyser reports **`W230`** with the resolved offset and the list
length so the mistake is obvious.

## Fix

Use `end` for the last element, positive indices inside the range, or
`llength` to guard the access.

```tcl
set first [lindex $xs 0]
set last  [lindex $xs end]
```

## How to suppress

Add `# noqa: W230` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [W231 — `lset` index out of range](kcs-diagnostic-w231-lset-index-out-of-range.md)
- [W232 — string index out of range](kcs-diagnostic-w232-string-index-out-of-range.md)

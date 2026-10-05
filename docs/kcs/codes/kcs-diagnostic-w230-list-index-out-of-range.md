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

These commands silently return the empty string, clamp the range, or
(for `lreplace`) prepend/append instead of replacing when the index is
out of bounds. That silent behaviour hides real bugs: the programmer
usually expected an element and will never see the error. `linsert` is
deliberately excluded — its clamp always produces a sensible result, so
flagging it would second-guess intent.

The analyser checks two shapes. The first is a constant index into a list
written as a literal in the same command, where it can count the elements:

- A plain integer like `-1` or `5`.
- An `end-N` expression where `N` is larger than the list length minus
  one (so the resolved offset is negative).

A constant index into a list held in a variable is not checked:
`lindex $xs 5` and `lindex $xs -1` are never flagged, even after
`set xs {a b c}`.

The second is an `lindex` whose index is a variable, into a list whose length
the analyser knows — a variable set from a literal list or from `[list …]` —
where every value the index may hold lies outside the list. A loop's counter
holds a known value after the loop when the analyser runs the loop to its
end:

```tcl
set xs {a b c}
for {set i 0} {$i < 5} {incr i} {}
puts [lindex $xs $i]   ;# i is 5 after the loop
```

The second shape is read for `lindex` alone, and not inside a quoted word.

## Example that triggers it

```tcl
set first [lindex {a b c} -1]    ;# want end, got ""
set tail  [lindex {a b c} end-5] ;# list only has 3 elements -> ""
set slice [lrange {a b c} 10 20]
```

The analyser reports **`W230`** with the resolved offset and the list
length so the mistake is obvious.

## Fix

Use `end` for the last element, positive indices inside the range, or
`llength` to guard the access.

```tcl
set first [lindex {a b c} 0]
set last  [lindex {a b c} end]
```

## How to suppress

Add `# noqa: W230` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W231 — `lset` index out of range](kcs-diagnostic-w231-lset-index-out-of-range.md)
- [W232 — string index out of range](kcs-diagnostic-w232-string-index-out-of-range.md)

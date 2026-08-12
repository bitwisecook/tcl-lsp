# KCS: W232 — Why does the analyser warn about a string index that is out of range?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, const-fold

## Profiles

default

## Question

Why does the analyser flag a constant index passed to `string index`,
`string range`, `string replace`, or `string insert` that falls outside
the string?

## Why

Every `string` command with an out-of-range index returns silently in
Tcl 9:

- `string index` returns the empty string.
- `string range` clamps to the valid slice (often empty).
- `string replace` is a no-op — the original string is returned.
- `string insert` clamps the insertion point to the start or end.

None of these raise an error. When the index is a literal, the
analyser can tell whether the expression underflows or overshoots
and reports it.

## Example that triggers it

```tcl
set first [string index "abc" -1]      ;# negative literal -> ""
set last  [string index "abc" end-5]   ;# only 3 chars -> ""
set mid   [string range "abc" end-99 -50]
set same  [string replace "abc" -1 -1 X]
set front [string insert "abc" -5 X]   ;# clamps to start
```

The analyser reports **`W232`** with the resolved offset and the
string length.

## Fix

Use indices inside `[0, [string length $s])` or use `end` / valid
`end-N` expressions. Guard with `string length` when the string may
be shorter than expected.

## How to suppress

Add `# noqa: W232` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W230 — list index silently out of range](kcs-diagnostic-w230-list-index-out-of-range.md)
- [W231 — `lset` index out of range](kcs-diagnostic-w231-lset-index-out-of-range.md)

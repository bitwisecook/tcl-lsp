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

Every one of these `string` subcommands returns silently when an index
is out of range:

- `string index` returns the empty string.
- `string range` clamps to the valid slice (often empty).
- `string replace` is a no-op — the original string is returned.
- `string insert` clamps the insertion point to the start or end
  (`string insert` exists from Tcl 9.0 onwards).

None of these raise an error. When the index is a literal, the
analyser can tell whether the expression underflows or overshoots
and reports it.

The string can be written in the command. It can also be held in a
variable whose value the analyser knows there, such as one set from a
literal. The command can stand on its own line or sit inside another
command, as in `puts [string index $s 9]`. An index held in a variable
whose value the analyser knows counts as a literal. Each access is
reported once. A string written as a command substitution in the call
itself, as in `string index [string repeat a 3] 9`, is not checked
(#2438).

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

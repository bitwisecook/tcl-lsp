# KCS: S101 — Why does the analyser warn about a shimmer inside a loop?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser flag a type-conversion (shimmer) inside a loop body?

## Why

Each loop iteration converts the value between types, multiplying the cost by the number of iterations.

## Symptoms

- A yellow squiggle under the variable use, with a message naming both types:
  "variable 'item' has numeric intrep but 'lindex' expects list (argument 1)".
  A value that changes type on the loop's back edge reads "'total' merges string
  and int at control-flow join" instead.

## Example that triggers it

```tcl
set items [list 1 2 3]
foreach item $items {
    puts [expr {$item + 0}]
    puts [lindex $item 0]
}
```

The analyser reports **`S101`** on the `lindex`: `item` is converted from
numeric to list on every iteration.

## Fix

```tcl
set items [list 1 2 3]
foreach item $items {
    set n [expr {$item + 0}]
    puts $n
    puts [lindex [list $item] 0]
}
```

Read each value as one type. `S101` is the in-loop sibling of
[`S100`](kcs-diagnostic-s100-shimmer-outside-loop.md); the conversion cost is
the same, multiplied by the iteration count.

## How to suppress

Add `# noqa: S101` on the line **above** the offending command, or apply
the "Suppress S101 with a noqa comment" quick fix offered on the
diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S100`, `S102`

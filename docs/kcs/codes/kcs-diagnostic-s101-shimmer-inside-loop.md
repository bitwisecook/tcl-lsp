# KCS: S101 — Why does the analyser warn about a shimmer inside a loop?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser flag a type-conversion (shimmer) inside a loop body?

## Why

Each loop iteration converts the value between types, multiplying the cost by the number of iterations.

## Symptoms

- A yellow squiggle appears under the variable use inside the loop, with the message "shimmer inside loop body".

## Example that triggers it

```tcl
foreach item $list {
    expr {$item + 0}
    string length $item
}
```

The analyser reports **`S101`** because `item` shimmers on every iteration.

## Fix

Extract the numeric value to a separate variable before the string use:

```tcl
foreach item $list {
    set item_num [expr {$item + 0}]
    string length $item
}
```

## How to suppress

Add `# noqa: S101` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer)
- Related codes: `S100`, `S102`

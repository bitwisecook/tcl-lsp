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

- Yellow squiggle under the variable use, with the message "shimmer inside loop body".

## Example that triggers it

```tcl
foreach item $list {
    expr {$item + 0}
    string length $item
}
```

The analyser reports **`S101`** because `item` shimmers on every iteration.

## Fix

```tcl
foreach item $list {
    set item_num [expr {$item + 0}]; string length $item
}
```

## How to suppress

Add `# noqa: S101` on the line **above** the offending command, or apply
the "Suppress S101 with a noqa comment" quick fix offered on the
diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S100`, `S102`

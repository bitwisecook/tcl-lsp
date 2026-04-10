# KCS: S100 — Why does the analyser warn about a shimmer outside a loop?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser flag a single type-conversion (shimmer) outside a loop?

## Why

Tcl converts the value's internal type each time the code runs, which wastes CPU time and can accumulate in hot paths.

## Symptoms

- A yellow squiggle appears under the variable use, with the message "value shimmers between types".

## Example that triggers it

```tcl
set x "42"
expr {$x + 1}
string length $x
```

The analyser reports **`S100`** because `x` is used as both an integer and a string.

## Fix

Use separate variables for numeric and string use:

```tcl
set x "42"
set x_num [expr {$x + 0}]
expr {$x_num + 1}; string length $x
```

## How to suppress

Add `# noqa: S100` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S101`, `S102`

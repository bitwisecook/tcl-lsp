# KCS: S102 — Why does the analyser warn about shimmer oscillation?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser flag a variable that oscillates between two types across loop iterations?

## Why

The value converts back and forth on every iteration, making the performance cost linear in the loop count.

## Symptoms

- Yellow squiggle under the variable, with the message "variable oscillates between types across iterations".

## Example that triggers it

```tcl
proc accumulate {} {
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}
```

The analyser reports **`S102`** because `x` alternates between integer and string types
on every pass through the loop.

## Fix

Give the two roles separate variables so neither one's intrep has to keep
flipping:

```tcl
proc accumulate {} {
    set x_num 0
    while {1} {
        set x_num [expr {$x_num + 1}]
        set x_str [string range $x_num 0 end]
    }
}
```

## How to suppress

Add `# noqa: S102` on the line **above** the offending command, or apply
the "Suppress S102 with a noqa comment" quick fix offered on the
diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S100`, `S101`

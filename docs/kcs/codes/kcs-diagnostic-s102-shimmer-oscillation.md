# KCS: S102 — Why does the analyser warn about shimmer oscillation?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser flag a variable that oscillates between two types across loop iterations?

## Why

The value converts back and forth on every iteration, making the performance cost linear in the loop count.

## Symptoms

- A yellow squiggle under the variable, with the message "'total' oscillates
  between string and int across loop iterations (thunking)". `S101` normally
  reports the same line as well.

## Example that triggers it

```tcl
set total 0
foreach item {1 2 3} {
    set total [expr {$total + 1}]
    set total [string range $total 0 end]
}
puts $total
```

The analyser reports **`S102`** because `total` alternates between integer and
string on every pass through the loop.

## Fix

Give the two roles separate variables so neither one's intrep has to keep
flipping:

```tcl
set total 0
foreach item {1 2 3} {
    set total [expr {$total + 1}]
}
puts [string range $total 0 end]
```

Keep one type inside the loop and convert once on the way out.

## How to suppress

Add `# noqa: S102` on the line **above** the offending command, or apply
the "Suppress S102 with a noqa comment" quick fix offered on the
diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S100`, `S101`

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

The value can convert back and forth on repeated iterations, making the performance cost grow with the loop count.

## Symptoms

- A yellow squiggle under the variable describing repeated conversion between
  representations. A source-only warning can say the conversion **may** repeat
  when it depends on the original handlers and unobserved variable cells.

## Example that triggers it

```tcl
set total 0
foreach item {1 2 3} {
    incr total
    lindex $total 0
}
puts $total
```

The analyser reports **`S102`** because `incr` uses an integer representation
and `lindex` converts the result to a list on each pass through the loop.

## Fix

Give the two roles separate variables so neither one's intrep has to keep
flipping:

```tcl
set total 0
foreach item {1 2 3} {
    incr total
}
puts [lindex $total 0]
```

Keep one type inside the loop and convert once on the way out.

## How to suppress

Add `# noqa: S102` on the line **above** the offending command. Some
warnings also offer a "Suppress S102 with a noqa comment" quick fix.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S100`, `S101`

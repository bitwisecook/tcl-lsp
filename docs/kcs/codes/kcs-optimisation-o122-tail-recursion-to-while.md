# KCS: O122 — Convert tail-recursive proc to iterative while loop

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, tail-call

## Profiles

full

## Question

What does O122 rewrite, and when does it fire?

## Why

An iterative loop has zero call overhead and cannot overflow the stack regardless of input size. This fires when a fully tail-recursive proc can be expressed as a `while` loop.

## Before

```tcl
proc sum {lst acc} {
  if {[llength $lst] == 0} { return $acc }
  tailcall sum [lrange $lst 1 end] [expr {$acc+[lindex $lst 0]}]
}
```

## After

```tcl
proc sum {lst acc} {
  while {[llength $lst] > 0} {
    set acc [expr {$acc+[lindex $lst 0]}]
    set lst [lrange $lst 1 end]
  }; return $acc
}
```

## Safety conditions

- Skipped when the proc contains multiple recursive call sites or uses `uplevel`, `upvar`, or other stack-sensitive commands.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Tail-call analysis](../../GLOSSARY.md#tail-call-optimisation)
- Related codes: `O121`, `O123`

# KCS: O123 — Detect non-tail recursion eligible for accumulator introduction (hint)

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, tail-call

## Profiles

full

## Question

What does O123 report, and when does it fire?

## Why

Adding an accumulator parameter can make a proc tail-recursive, enabling O121 or O122. This hint fires when the optimiser detects a recursive call that is not in tail position but could be restructured.

## Before

```tcl
proc sum {lst} {
  if {[llength $lst] == 0} { return 0 }
  expr {[lindex $lst 0] + [sum [lrange $lst 1 end]]}
}
```

## After

Hint suggests adding an `acc` parameter so the proc becomes tail-recursive.

## Safety conditions

- Skipped when the recursive call is already in tail position.
- Skipped when the combining operation is not associative, making accumulator introduction unsafe.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Tail-call analysis](../../GLOSSARY.md#tail-call-optimisation)
- Related codes: `O121`, `O122`

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
proc fact {n} {
    if {$n <= 1} { return 1 } else { return [expr {$n * [fact [expr {$n - 1}]]}] }
}
```

## After

O123 rewrites nothing. It reports *"Proc 'fact' is a candidate for
accumulator-style rewriting"* — the recursive call sits inside an expression,
so adding an accumulator parameter would put it in tail position and let
[O121](kcs-optimisation-o121-tailcall-rewrite.md) or
[O122](kcs-optimisation-o122-tail-recursion-to-while.md) take over.

## Safety conditions

- Skipped when the recursive call is already in tail position.
- Skipped when the combining operation is not associative, making accumulator introduction unsafe.
- Skipped for tree recursion — a proc that calls itself more than once per branch is not an accumulator candidate.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Tail-call analysis](../../GLOSSARY.md#tail-call-optimisation)
- Related codes: `O121`, `O122`

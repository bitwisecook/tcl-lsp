# KCS: O121 — Rewrite self-recursive tail calls to tailcall

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, tail-call

## Profiles

full

## Question

What does O121 rewrite, and when does it fire?

## Why

`tailcall` avoids growing the call stack on every recursive call, preventing stack overflow on deep recursion. The rewrite fires when a proc's last action is a call to itself.

## Before

```tcl
proc fact {n acc} {
  if {$n <= 1} { return $acc }
  return [fact [expr {$n-1}] [expr {$n*$acc}]]
}
```

## After

```tcl
proc fact {n acc} {
  if {$n <= 1} { return $acc }
  tailcall fact [expr {$n-1}] [expr {$n*$acc}]
}
```

## Safety conditions

- Skipped when the recursive call is not in [tail position](../../GLOSSARY.md#tail-position).
- Skipped when the call is wrapped in a `catch` or `try` block.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Tail-call analysis](../../GLOSSARY.md#tail-call-optimisation)
- Related codes: `O122`, `O123`

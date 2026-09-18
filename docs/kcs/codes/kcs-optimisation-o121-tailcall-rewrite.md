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
proc walk {node acc} {
  if {$node eq ""} { return $acc }
  set acc [combine $acc [walk [left $node] {}]]
  return [walk [right $node] $acc]
}
```

## After

```tcl
proc walk {node acc} {
  if {$node eq ""} { return $acc }
  set acc [combine $acc [walk [left $node] {}]]
  tailcall walk [right $node] $acc
}
```

The call is rewritten as written, so its arguments — including braced,
quoted, and `{*}`-expanded words — reach the `tailcall` unchanged. The first,
non-tail `walk` is left alone, and it is what keeps [O122](kcs-optimisation-o122-tail-recursion-to-while.md)
away: were every self-call in tail position, O122 would convert the whole proc
to a loop and supersede this rewrite.

## Safety conditions

- Skipped when the recursive call is not in [tail position](../../GLOSSARY.md#tail-position).
- Skipped when the call is wrapped in a `catch` or `try` block.
- Skipped before Tcl 8.6, which has no `tailcall` (TIP 327). The O122 loop conversion still applies there.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Tail-call analysis](../../GLOSSARY.md#tail-call-optimisation)
- Related codes: `O122`, `O123`

# KCS: I231 — Why does the analyser say a `switch` arm never runs?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, sccp

## Profiles

default

## Question

Why does the analyser report that one of my `switch` arms is unreachable — that
the dispatch value is "always" a particular case, so the other arms can never
match?

## Why

When the value a `switch` dispatches on is a compile-time constant, the analyser
knows exactly which arm will match, so the remaining arms are dead code:

```tcl
switch -- 1 {
    1       { puts "one" }
    2       { puts "two" }
    default { puts "other" }
}
```

The subject is the literal `1`, so the `1` arm always matches and the `2` and
`default` arms can never run. The analyser folds the dispatch to its known
result and reports **`I231`** on the constant arm. The same thing happens when
an arm's guard is itself constant (e.g. an arm condition that is provably always
true or always false).

A constant `if` / `elseif` chain reports its sibling code
[`I230`](kcs-diagnostic-i230-constant-existence-check.md) instead; `I231` is the
`switch`-specific variant. Once a branch is known dead, the optimiser can drop
it ([`O107`](kcs-optimisation-o107-unreachable-dead-code.md)).

## Fix

Dispatch on a runtime value so the matching arm is no longer fixed at compile
time:

```tcl
switch -- $mode {
    1       { puts "one" }
    2       { puts "two" }
    default { puts "other" }
}
```

With a non-constant subject the matched arm is no longer known, and `I231`
disappears. If an arm really is dead, delete it.

## How to suppress

Add `# noqa: I231` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [I230 — constant existence / branch check](kcs-diagnostic-i230-constant-existence-check.md)
- [O107 — unreachable dead code](kcs-optimisation-o107-unreachable-dead-code.md)
- [W240 — loop condition constant false](kcs-diagnostic-w240-loop-constant-false.md)

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

When the value a `switch` dispatches on is a compile-time constant — a literal,
or a variable the analyser proves holds one value at the `switch` — the analyser
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

### Every form of `switch`

The same holds for a `-glob`, `-regexp` or `-nocase` switch, one with a
fall-through arm (`a -`), and Tcl's `case`, over a subject the analyser proves.
It works out which arm the command would select, by the command's own matching
rules, and reports each arm that never runs at its pattern:

```tcl
set acc ""
append acc foo
append acc bar
switch -glob -- $acc {
    baz     { puts never }    ;# I231: Switch arm 'baz' is never selected
    default { puts always }
}
```

An arm that passes its body on with `-` is judged by the body it leads to, so
the alternates of a body that runs are not reported, and `default` is never
reported. The analyser makes no selection when it cannot be sure which arm
runs: a subject that varies, a pattern it cannot decide, a `-nocase` under a
profile that may be Tcl 8.4, a quoted or braced `-` body in the
separate-words form under a profile that may be Tcl 9.1, whose compiled and
interpreted paths read that word differently, or a subject that starts with `-`
where `switch` may read it as an option unless `--` ends them: under a profile
that may be Tcl 8.4, which reads every leading word that starts with `-` as an
option (`set x -glob; switch $x {…}` is `bad option` there and selects the
`-glob` arm from 8.5), and on every release when the arms are written as
pattern and body words (`set x -glob; switch $x a {…} default {…}` is an
error).

These forms have no block of their own for an arm, so nothing is dropped as
unreachable code ([`O107`](kcs-optimisation-o107-unreachable-dead-code.md) does
not fire); the optimiser instead folds the whole `switch` down to the body that
runs (`O112`). The exact form without a fall-through arm is a chain of
comparisons, so its dead arms are unreachable blocks and `O107` removes their
bodies too — unless its subject is a variable with no `--` before it where
`switch` may read it as an option, where the statement is kept whole and judged
like the forms above. The words of the comparisons are read by their values, the way
Tcl reads them: `a\nb`, `"a\nb"` and a braced list element holding a newline are
one string.

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

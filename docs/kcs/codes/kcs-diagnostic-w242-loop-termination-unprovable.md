# KCS: W242 — Why does the analyser hint that my loop may not terminate?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

opt-in — `tclLsp.diagnostics.W242` defaults to `false`. Most real loops end
on a condition the analyser cannot follow, so the hint would fire far more
often than it would help.

## Question

Why does the analyser hint that it cannot prove a `while` or `for`
loop terminates?

## Why

W242 is the counterpart to [W241](kcs-diagnostic-w241-loop-provably-infinite.md).
W241 fires when the analyser can **prove** the loop runs forever.
W242 fires when the analyser can prove neither termination nor
non-termination: the counter variable in the condition is never
visibly assigned by the step or body. An assignment in a body word written
bare or quoted counts (`if {$c} "incr n"`), as one in a braced word does.

It is reported at hint severity. Turn it on when you want every loop
whose termination is not obvious from the surrounding source flagged.

A loop whose condition the analyser can decide is reported as W240 (the
condition is false when the loop is reached) or W241 (it is true at every
test and nothing leaves the loop) instead, so W242 is for the conditions it
cannot decide — a parameter, a value read from elsewhere.

## Example that triggers it

```tcl
while {$running < 10} {
    process_event
}
```

`$running` appears in the condition but nothing in the body visibly
updates it. Either `process_event` has a side effect the analyser
cannot see (then suppress the hint) or the loop is buggy.

## Fix

Either modify the counter in the loop or add a `break` guard.

```tcl
while {$running < 10} {
    incr running
    process_event
}
```

## How to enable

Set `tclLsp.diagnostics.W242: true` in your editor's configuration.

## How to suppress

Add `# noqa: W242` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W240 — loop condition is constant false](kcs-diagnostic-w240-loop-constant-false.md)
- [W241 — provably infinite loop](kcs-diagnostic-w241-loop-provably-infinite.md)

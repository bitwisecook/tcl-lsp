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
non-termination and nothing in the loop may assign the variable in the
condition. It reads what may assign it from the dataflow it builds for the
code around the loop: a write in the step or the body, in a braced, bare or
quoted word alike (`if {$c} "incr n"` assigns `n`), in a `[…]` word, by a
`foreach` binder or by a procedure that sets the caller's variable with
`upvar` or `uplevel`; a call it cannot see into — a command the file does
not define, a computed command, a `source` — and a callback the file stores,
either of which may assign it. Any of them keeps the hint away, and so does a
loop whose code the analyser does not follow, such as one inside a `catch`
body.

It is reported at hint severity. Turn it on when you want every loop
whose termination is not obvious from the surrounding source flagged.

A loop whose condition the analyser can decide is reported as W240 (the
condition is false when the loop is reached) or W241 (it is true at every
test and nothing leaves the loop) instead, so W242 is for the conditions it
cannot decide — a parameter, a value read from elsewhere.

## Example that triggers it

```tcl
while {$running < 10} {
    puts "waiting"
}
```

`$running` appears in the condition but nothing in the loop updates it:
`puts` assigns no variable. Had the body called a command the file does not
define, such as `process_event`, the analyser could not rule out that it sets
`running`, and would not hint.

## Fix

Either modify the counter in the loop or add a `break` guard.

```tcl
while {$running < 10} {
    incr running
    puts "waiting"
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

# KCS: W242 — Why does the analyser hint that my loop may not terminate?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

opt-in — disabled by default

## Question

Why does the analyser hint that it cannot prove a `while` or `for`
loop terminates?

## Why

W242 is the counterpart to [W241](kcs-diagnostic-w241-loop-provably-infinite.md).
W241 fires when the analyser can **prove** the loop runs forever.
W242 fires when the analyser can prove neither termination nor
non-termination: the counter variable in the condition is never
visibly assigned by the step or body.

The check is a HINT (severity) and is **off by default** to keep
noise low. Enable it when you want the analyser to flag any loop
whose termination is not obvious from the surrounding source.

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

Add `# noqa: W242` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [W240 — loop condition is constant false](kcs-diagnostic-w240-loop-constant-false.md)
- [W241 — provably infinite loop](kcs-diagnostic-w241-loop-provably-infinite.md)

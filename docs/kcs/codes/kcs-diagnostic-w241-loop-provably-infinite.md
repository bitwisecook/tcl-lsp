# KCS: W241 — Why does the analyser warn that my loop is provably infinite?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default

## Question

Why does the analyser flag a `while` or `for` as provably infinite?

## Why

The analyser can see enough of the loop shape to prove it will never
terminate. It reports **`W241`** in these cases:

- A `while {1}` / `while {true}` whose body never leaves the loop —
  no `break`, and nothing that terminates the enclosing block or frame
  (`return` / `error` / `exit` / `throw` / `tailcall`). A `continue`
  does *not* count: it restarts the loop, so the loop is still infinite.
- A `for {set v INT} {$v OP INT} {incr v INT}` where the
  counter cannot reach the bound:
  - `incr v 0` — the counter never changes.
  - The counter moves *away* from the bound (`$v < N` with a
    negative step, `$v > N` with a positive step).
  - The counter skips the bound (`$v != N` with a step whose
    direction or magnitude never lands exactly on `N`).

If the body assigns the counter itself (`set v ...`, nested
`incr v`, `lset`, ...) the analyser backs off — it cannot
reason about arbitrary rewrites.

## Example that triggers it

```tcl
while {1} {
    puts "forever"
}

for {set i 0} {$i < 10} {incr i 0} {
    puts "step is zero"
}

for {set i 0} {$i < 10} {incr i -1} {
    puts "wrong direction"
}

for {set i 0} {$i != 10} {incr i 3} {
    puts "skips 10"
}
```

## Fix

Add a `break` / `return` path, fix the step direction, or use `<` /
`<=` in place of `!=` so the loop ends cleanly.

```tcl
for {set i 0} {$i < 10} {incr i} {
    puts $i
}
```

## How to suppress

Add `# noqa: W241` at the end of the offending line. (Genuine
event-loop servers that really do loop forever are the intended
suppression target.)

## Related

- [KCS codes index](README.md)
- [W240 — loop condition is constant false](kcs-diagnostic-w240-loop-constant-false.md)
- [W242 — loop termination not provable](kcs-diagnostic-w242-loop-termination-unprovable.md)
- `IRULE5003` — iRules loop condition `!= 0` with decrement

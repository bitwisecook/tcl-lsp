# KCS: `yield` used to report "C stack busy" inside `try`, a bare `apply` call, or a value-consumed `lmap`

> **Audience:** Contributor
> **Type:** Issue

## Applies to

tcl-lsp CLI

## Question

Why did `yield` (inside a coroutine) error `cannot yield: C stack busy` when
it was reached inside a `try` body/handler/`finally`, a bare `apply {lambda}`
call, or an `lmap`/`foreach` whose result was consumed (e.g. `set r [lmap x
{1 2} { yield $x }]`), when tclsh 9.0 completes those scripts successfully?

## Symptoms

- `coroutine c gen; puts [c]` errors `cannot yield: C stack busy` where `gen`
  contains a `yield` inside `try { ... }`.
- Same error for `apply {{} { yield a }}` called as a plain command from
  inside a coroutine's body (as opposed to `coroutine c apply {lambda}`,
  which already worked).
- Same error for `set r [lmap x {...} { yield $x }]` — the *bare* statement
  form `lmap x {...} { yield $x }` (result discarded) already worked; only
  the value-consuming form failed.

## Answer

The VM's coroutine machinery (`RUST_ISSUE_008`) suspends a `yield` by
freezing the *explicit* activation stack (`Vec<Frame>`) the bytecode
trampoline drives. A construct that instead re-enters the evaluator via a
*nested* nested-drive call (`Vm::eval_source`) runs on the *native* Rust
call stack, which the coroutine machinery cannot freeze — hence the
boundary error.

`try`, a bare `apply`, and the `foreach`/`lmap` runtime fallback (used when
`lmap`'s result is consumed or its body branches) all used to run their
bodies through exactly that nested-drive path. Each was converted to the
same pattern `eval`/`uplevel 0`/`catch` already used: defer the body to a
`Vm.pending_*` slot, drained by the trampoline into a dedicated activation
pushed onto the *explicit* stack instead:

- **`apply`** binds its lambda to a temporary proc (as before) and defers
  the call via `pending_eval` (the same mechanism `eval` uses), with the
  temporary proc torn down once the pushed frame completes.
- **A consumed/branching `lmap`/`foreach`** defers its whole iteration to a
  new scanner-driven **each-loop activation**, modelled on `subst`'s scanner
  frame: each iteration's body runs as a yieldable child frame, folded back
  by the loop's own collect/continue/break rules.
- **`try`** became a small state machine (`TryPhase::Body` → `Handler` →
  `Finally`) driven by `advance_try`, called from `Vm::unwind` each time a
  phase's activation completes. Each phase (body, the matched handler,
  `finally`) is its own pushed activation.

### Loop completions

The bytecode emitter now gives every inline `foreach` body dedicated
landing pads for its paired `FOREACH_STEP` and `FOREACH_END` opcodes. A
completion returned by a called proc, or passed transparently through an
unhandled `try`, therefore advances or exits the enclosing loop instead of
restarting its iterator or escaping the loop:

```tcl
proc skip {} { return -code continue }
foreach i {1 2 3} {
    if {$i == 2} { skip }
    puts $i
}
# => 1
# => 3
```

The same result holds when `skip` is replaced by an unhandled `continue`
inside a `try` body. A direct nested `foreach` or `lmap` still uses the
runtime-loop boundary because its two iterator back-edges need a dedicated
control-flow graph representation. It is semantically transparent, including
for `break`, `continue`, `return`, and `yield`; it is a correctness boundary,
not a user-visible limitation.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- `rust-issues/RUST_ISSUE_008.md` — the full coroutine yieldability history

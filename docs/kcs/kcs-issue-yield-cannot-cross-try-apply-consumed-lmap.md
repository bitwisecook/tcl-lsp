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

### Known residual gap

A `try`'s body being transparent to a *bare* `break`/`continue` (per TIP
329 — propagating to the caller's enclosing loop when nothing inside `try`
handles it) still does not work for a simple, single-command loop body: it
hits a pre-existing compiler gap where the loop's `loop_targets` entry
redirects to the wrong instruction and would hang rather than resume the
next iteration, so `try`'s activation deliberately does not attempt the
`eval`-style transparent redirect there. This is not a regression — the
same gap already makes a bare `return -code continue` from a called proc
report `invoked "continue" outside of a loop` instead of correctly
continuing, and no combination of yieldable constructs previously reached
this path at all (yield could not even suspend `try` before).

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- `rust-issues/RUST_ISSUE_008.md` — the full coroutine yieldability history

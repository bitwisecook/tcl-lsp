# RUST_ISSUE_008: coroutines/`yield`/`yieldto` error on the wasm32 target though the native runtime implements them; the VM lacks them entirely

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `WASM backend` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

WASM backend — coroutines/`yield`/`yieldto` error on the wasm32 target though the native runtime implements them; the VM lacks them entirely.
cmd_coro.rs:445-473 cfg-gates the wasm32 impls to `set_error("coroutines are not supported in the single-threaded wasm build")` (native at :391-438). `yieldto` errors on BOTH targets (:412 "not yet implemented"). The bytecode VM has zero coroutine support. Across backends: native ✓, WASM target ✗ (runtime error), VM ✗ (missing). Confidence: high

## Progress — VM coroutines (Phase 1, partial)

The bytecode VM now has coroutines, built on the explicit-stack NRE trampoline
(`rust/tcl-vm/src/cmd_coro.rs`) rather than the runtime's OS-thread design: a
suspended coroutine's continuation is its **frozen `Vec<Frame>` activation stack
plus a saved per-flow context** (`ParkedFlow`), swapped in/out by
`Vm::swap_flow` — pure data, no threads, `forbid(unsafe)` kept. `yield` records a
request that `dispatch_words` turns into a new `Tick::Suspend` (mirroring the
`tailcall` → `Tick::Tailcall` plumbing; no compiler/bytecode change).

Working and oracle-checked against tclsh 9.0.4
(`rust/tcl-vm/tests/cmd_coro_e2e.rs`, 17 tests): `coroutine`/`yield`,
generator-style bodies (`yield $x` in `foreach`/`while`/nested proc calls),
**command substitution** (`set arg [yield $result]` — the resume-value idiom —
and `cmd [yield]` argument position), **`coroutine c apply {lambda}`** (the
anonymous-generator form), independent interleaved coroutines, `[info
coroutine]`, the resume command, `rename $coro {}` teardown + rename-to-new, the
already-running guard, and `yield`/`yieldto` outside a coroutine. Boundary errors
match C's `cannot yield: C stack busy`, detected via a `Vm::activation_depth`
re-entry counter.

**Command substitution is now yieldable.** A whole-word `[…]` compiles to an
inline `INVOKE` on the explicit activation stack (matching C Tcl, which never
runs a whole-word substitution through a recursive `Tcl_EvalObjEx`) rather than
the runtime `subst_word` fallback, which re-entered the evaluator on the native
stack. This closes the gating item for real-world generators. (En route it also
fixed a pre-existing codegen bug: a namespace-qualified bare var `$::x`/`$ns::v`
inside a command substitution was pushed as a literal instead of loaded, so e.g.
`[string length $::x]` measured the string `"$::x"`.)

**`coroutine … apply {lambda}` is yieldable.** `cmd_coroutine` binds the lambda
to an internal proc and runs *that* (`lambdaProc arg…`) as the body, so the
lambda executes on the coroutine's explicit stack; the proc is torn down with the
coroutine. (The lambda-parse logic is shared with `apply` via `build_lambda_proc`;
`apply`'s own behaviour is unchanged.)

## Progress — event loop (Phase 2)

The VM now has a minimal but faithful single-threaded event loop
(`rust/tcl-vm/src/cmd_event.rs`, ported from the tree-walker's `cmd_event.rs`):
`after`/`vwait`/`update` over an `EventQueue` (deadline-ordered timers + a FIFO
idle queue) on the `Vm`. This is the scheduler half of coroutines — `after 0
$coro` schedules a resume and `vwait`/`update` drives it. Event-handler errors go
to the `bgerror` handler (or stderr). Oracle-checked in
`rust/tcl-vm/tests/cmd_event_e2e.rs` (12 tests): `after 0`/`after idle`,
scheduling vs deadline order, `after cancel` by id/script, `update idletasks`,
the `after#0` id, the error messages, and the coroutine-driver pattern.

En route, a startup-init bug was fixed: the interp's bootstrap ran `set
::auto_path [list [info library]]`, which errors when no Tcl library is
configured — leaving `::auto_path` unset (so the `unknown`/auto-load path failed
with `can't read "::auto_path"`) and polluting `::errorInfo`. The init now sets
`::auto_path` to `{}` and appends `[info library]` under a `catch`.

**Remaining (still Open):**
- A `yield` reached across a host re-entry the VM runs on the **native Rust
  stack** — `catch`/`uplevel`/`eval`, and `apply` in an *arbitrary* position
  (not the `coroutine … apply` form, which is handled) — still errors `cannot
  yield: C stack busy` instead of yielding. C Tcl makes those NR-enabled; making
  them re-enter the explicit trampoline is the remaining yieldable-surface work.
- `yieldto` beyond the outside-a-coroutine error; the creating-namespace/`info
  level` refinements.
- The **wasm32** runtime coroutines (asyncify or VM-on-wasm) and the `thread`
  package (Phase 3) — untouched.

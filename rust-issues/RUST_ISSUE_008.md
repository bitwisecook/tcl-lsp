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
(`rust/tcl-vm/tests/cmd_coro_e2e.rs`, 11 tests): `coroutine`/`yield`,
generator-style bodies (`yield $x` in `foreach`/`while`/nested proc calls),
independent interleaved coroutines, `[info coroutine]`, the resume command,
`rename $coro {}` teardown + rename-to-new, the already-running guard, and
`yield`/`yieldto` outside a coroutine. Boundary errors match C's
`cannot yield: C stack busy`, detected via a `Vm::activation_depth` re-entry
counter.

**Remaining (still Open):**
- A `yield` reached across a **host re-entry** — command substitution
  `set x [yield]` (the resume-value idiom), `apply`, `catch`/`uplevel`/`eval` —
  errors `cannot yield: C stack busy` instead of yielding, because those
  constructs re-enter `run_activation` on the host stack rather than staying on
  the explicit `acts` stack (C Tcl makes them NR-enabled). Extending the
  yieldable surface is the next step and the gating item for real-world usage.
- `yieldto` beyond the outside-a-coroutine error; the creating-namespace/`info
  level` refinements; the `after 0 $coro` event-loop driver (Phase 2).
- The **wasm32** runtime coroutines (asyncify or VM-on-wasm) and the `thread`
  package (Phase 3) — untouched.

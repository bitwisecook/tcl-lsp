# RUST_ISSUE_008: coroutines/`yield`/`yieldto` error on the wasm32 target though the native runtime implements them; the VM lacks them entirely

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `WASM backend` |
| **Status** | Substantially resolved (VM coroutines + event loop + thread package; coroutines proven on wasm32 via the VM). Open tail: wiring the VM as the primary wasm compile backend, and VM `yield` across `catch`/`uplevel`/`eval`. |
| **Verification** | Oracle-checked against tclsh 9.0.4 (coroutine/event-loop suites) + a Node-executed wasm32 coroutine test; thread package via deterministic concurrency tests (no threaded oracle). |

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

## Progress — thread package (Phase 3)

The VM has a **real, shared-nothing `thread` package** (`rust/tcl-vm/src/
cmd_thread.rs`) — true OS-thread parallelism *without* making `Value`/`Vm`
`Send`. Each worker builds its **own** `Vm` inside the spawn closure (the `Vm`
never crosses a thread boundary, so `!Send`/`Rc` is fine); the only `Send + Sync`
surface is a small `Arc<Shared>` block: the worker registry (id → job channel),
the `tsv` store, a **`Send` compile-service factory** each worker calls to build
its compiler, and a `Send` output sink. `forbid(unsafe)` is kept — no `unsafe
impl Send` (the tree-walker's shortcut); the type system carries the safety.

Commands: `thread::create`/`send` (`-async`)/`wait`/`release`/`id`/`exists`/
`names`/`errorproc`, and `tsv::{set,get,exists,unset,incr,append,lappend,keys,
names}`. `thread::send` serializes a script to the target's channel and (unless
`-async`) blocks for its result; `thread::wait` is the worker message loop.
`tcl_platform(threaded)` is now honest — `0` on a bare VM, `1` once the embedder
(`tcl-vm-cli`) calls `Vm::enable_threads`.

No oracle: the reference tclsh 9.0.4 is a **non-threaded** build (no `Thread`
package, `tcl_platform(threaded)` unset), so this subsystem is validated by
deterministic concurrency tests (`rust/tcl-vm/tests/cmd_thread_e2e.rs`, 12 tests:
sync/async send, per-worker isolation, worker-error propagation, exists/names/
release, an atomic 4-thread `tsv::incr` counter totalling 1000, and the `tsv::*`
element operations), with semantics per the Tcl `Thread` package docs.

## Progress — coroutines on wasm32 (Phase 4)

The VM's coroutines are **pure data** (a frozen `Vec<Frame>` + saved flow, no OS
threads, no `unsafe`), so — as the plan anticipated — they are wasm-portable
without asyncify: the path to wasm coroutines is *the VM running on wasm32*. This
is proven end to end: the whole compile→run pipeline (`tcl-vm` + `tcl-compiler`)
builds for `wasm32-unknown-unknown`, and `rust/tcl-vm/tests/wasm_coro_e2e.rs`
generates a tiny `cdylib` over this workspace's crates, builds it to wasm32, and
runs it under **Node**'s `WebAssembly` API (no imports, no WASI). Three coroutine
scripts return the same tclsh-9.0.4 oracle values as on native: a `foreach`
generator with yieldable `[c]` command substitution → `234`, the `set n [yield
$sum]` resume-value idiom → `51518`, and `coroutine … apply {lambda}` → `809`.
The test skips cleanly without the wasm32 target or `node`.

One portability fix landed with it: `bootstrap_globals` populated the `env` array
via `std::env::vars()`, which the wasm32-unknown-unknown std shim aborts on; it is
now `cfg`-gated off that target (an empty `env`), leaving native/WASI unchanged.

This supersedes the runtime tree-walker's OS-thread-per-coroutine wasm stub (the
plan's asyncify alternative): the VM delivers working wasm coroutines directly.

**Remaining (still Open):**
- A `yield` reached across a host re-entry the VM runs on the **native Rust
  stack** — `catch`/`uplevel`/`eval`, and `apply` in an *arbitrary* position
  (not the `coroutine … apply` form, which is handled) — still errors `cannot
  yield: C stack busy` instead of yielding. C Tcl makes those NR-enabled; making
  them re-enter the explicit trampoline is the remaining yieldable-surface work.
- `yieldto` beyond the outside-a-coroutine error; the creating-namespace/`info
  level` refinements.
- Thread-package extras not yet modelled: `thread::mutex`/`cond`/`rwmutex` and
  `tpool::*` (the sync-send + `tsv` model already gives safe coordination), and
  the `tcl-registry` `Thread`-package `CommandSpec`s (LSP metadata; the runtime
  and the `RUST_ISSUE_006` core-backing gate do not require them). The `thread`
  package is a native-only VM feature (it needs OS threads); on wasm the VM runs
  single-threaded with coroutines, matching the plan's per-backend model.
- Wiring the VM as the primary **wasm compile target** (so the compiler emits
  VM-on-wasm rather than the tree-walker C-ABI) is a larger, separate migration;
  the coroutine capability it would carry is already proven here.

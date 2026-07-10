# RUST_ISSUE_008: coroutines/`yield`/`yieldto` error on the wasm32 target though the native runtime implements them; the VM lacks them entirely

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `WASM backend` |
| **Status** | Substantially resolved (VM coroutines incl. `coroprobe`/`coroinject`/`corotype`/`yieldto` + event loop + thread package; coroutines proven on wasm32 via the VM). C `coroutine.test` runs the VM at **50/77** (3 skipped, 24 remaining are the documented host-re-entry divergence + unimplemented `info frame`/`interp create`). Open tail: wiring the VM as the primary wasm compile backend, and VM `yield` across `catch`/`uplevel`/`eval`/`subst`. |
| **Verification** | Oracle-checked against tclsh 9.0.4 (coroutine/event-loop suites, 26 `cmd_coro_e2e` tests) + the real C `tests/coroutine.test` through the VM (50/77) + a Node-executed wasm32 coroutine test; thread package via deterministic concurrency tests (`thread.test` is `testthread`-gated → N/A; no threaded oracle). |

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
(`rust/tcl-vm/tests/cmd_coro_e2e.rs`, 26 tests): `coroutine`/`yield`,
generator-style bodies (`yield $x` in `foreach`/`while`/nested proc calls),
**command substitution** (`set arg [yield $result]` — the resume-value idiom —
and `cmd [yield]` argument position), **`coroutine c apply {lambda}`** (the
anonymous-generator form), independent interleaved coroutines, `[info
coroutine]`, the resume command, `rename $coro {}` teardown + rename-to-new, the
already-running guard, and `yield`/`yieldto` outside a coroutine. Boundary errors
match C's `cannot yield: C stack busy`, detected via a `Vm::activation_depth`
re-entry counter.

The TCL90 introspection/steering commands are implemented too: **`coroprobe`**
(evaluate a command in a *suspended* coroutine's own frame without resuming it),
**`coroinject`** (schedule a command to run in the coroutine at its next resume,
transforming what the parked `yield` returns), **`::tcl::unsupported::corotype`**
(report `active`/`yield`/`yieldto`), and **`yieldto`** delivering its N resume
args as a list. Further C-parity fixes: a body finishing with `return -code N`
for a non-standard N surfaces `Code::Other(N)` to the resumer; `[info coroutine]`
is empty once a coroutine deletes its own command; a coroutine's initial command
resolves in the namespace where `coroutine` was called; and **unset variable
traces fire on frame teardown** — both a normal proc return and coroutine
teardown (completion or deletion of a suspended coroutine) now unset a frame's
locals and fire their traces, as C does.

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

## Progress — C `coroutine.test` / `thread.test` harnesses

The actual C Tcl 9.0.4 `tests/coroutine.test` runs through the VM (bytecode) via
tcltest: **50 / 77 passing, 3 skipped** (the `testnrelevels`/`memory` constraints
gate C-only test commands the VM does not provide). The 24 remaining failures are
the design divergence, not regressions:

- **`yield`/`yieldto` cannot cross a host re-entry the VM runs on the native Rust
  stack** (13): `uplevel` (1.7/1.8/1.12), `eval` (1.9/1.10), `subst`
  (1.13/1.14), `lmap`-wrapped yield/yieldto (9.2, 10.1/10.2/10.3), `try` +
  event loop (7.14), and mutual `yieldto` whose `[yieldto …]` sits in a
  command-substitution argument slot that lowers to the runtime `subst_word`
  fallback rather than an inline `INVOKE` (7.3, 12.1). C Tcl NR-enables these;
  the VM reports the real `cannot yield: C stack busy` as an ordinary catchable
  error. Making every such construct re-enter the explicit trampoline is the
  open yieldable-surface work.
- **Introspection/embedding features the VM does not implement** (11): `info
  frame` (3.2/3.6/3.7/10.9), child interpreters `interp create` (7.7/9.9/12.1),
  and detecting a coroutine whose namespace was deleted mid-suspend to raise
  `yieldto called in deleted namespace` (7.8–7.11).

The C `thread.test` (52 tests) is **not applicable** to the VM's thread package:
every test is gated on the `testthread`/`thread` constraints, which require the
core `tcl::test` `testthread` command and a `Thread`-package/threaded build — a
different design from the VM's shared-nothing `Thread`-extension-style package
(Phase 3). Run through the VM, all 52 skip. The VM's threading is instead
validated by the native `cmd_thread_e2e.rs` suite (per the `Thread` package docs;
the reference tclsh is non-threaded, so there is no threaded oracle).

**Remaining (still Open):**
- A `yield`/`yieldto` reached across a host re-entry the VM runs on the **native
  Rust stack** — `catch`/`uplevel`/`eval`/`subst`/`lmap`/`try`, `apply` in an
  *arbitrary* position (not the `coroutine … apply` form, which is handled), and
  a `[yieldto …]` command substitution in an argument slot that lowers to runtime
  `subst_word` — still errors `cannot yield: C stack busy` instead of yielding. C
  Tcl makes those NR-enabled; making every such construct re-enter the explicit
  trampoline is the remaining yieldable-surface work (see the harness section:
  ~13 of the 24 `coroutine.test` failures). `yieldto` itself, the
  creating-namespace command resolution, and `info level`/`info coroutine` are
  done.
- Introspection/embedding gaps behind the other ~11 `coroutine.test` failures:
  `info frame`, child interpreters (`interp create`), and raising `yieldto
  called in deleted namespace` when a coroutine's namespace is deleted while it
  is suspended.
- Thread-package extras not yet modelled: `thread::mutex`/`cond`/`rwmutex` and
  `tpool::*` (the sync-send + `tsv` model already gives safe coordination), and
  the `tcl-registry` `Thread`-package `CommandSpec`s (LSP metadata; the runtime
  and the `RUST_ISSUE_006` core-backing gate do not require them). The `thread`
  package is a native-only VM feature (it needs OS threads); on wasm the VM runs
  single-threaded with coroutines, matching the plan's per-backend model.
- Wiring the VM as the primary **wasm compile target** (so the compiler emits
  VM-on-wasm rather than the tree-walker C-ABI) is a larger, separate migration;
  the coroutine capability it would carry is already proven here.

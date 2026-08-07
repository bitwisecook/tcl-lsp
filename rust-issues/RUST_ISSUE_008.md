# RUST_ISSUE_008: coroutines/`yield`/`yieldto` error on the wasm32 target though the native runtime implements them; the VM lacks them entirely

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `WASM backend` |
| **Status** | Substantially resolved (VM coroutines incl. `coroprobe`/`coroinject`/`corotype`/`yieldto` + event loop + thread package incl. `mutex`/`cond`/`rwmutex`/`tpool`; coroutines proven on wasm32 via the VM, now the **default `tcl compwasm` backend** shipping a generic `vm.wasm` runner; `yield` now crosses `eval`/`uplevel 0`/`catch`, a straight-line `lmap` (the inline collecting loop), and `subst` (a scanner-driven subst frame) on the explicit stack). C `coroutine.test` runs the VM at **60/77** (3 skipped; the lmap-in-`apply` 9.2/10.1/10.3 and the `subst`-as-coroutine 1.13/1.14 now pass; 10.2 still diverges on coroinject stacking order). Open tail: a `[yieldto …]` in a command-substitution *argument* slot (7.3/12.1, lowers to runtime `subst_word`), `lmap` in a *consumed*/branching position, and `try`/`apply` — now tracked as **GitHub issue #1311** with standalone repros for the last three. **Correction (2026-08-07):** `lsort -command` was listed here as an open barrier and is not one — C Tcl refuses it too (`tclsh9.0` and `tclvm` both raise `cannot yield: C stack busy` for a `yield` inside an `lsort -command` comparator), so that is parity, not a gap. |
| **Verification** | Oracle-checked against tclsh 9.0.4 (coroutine/event-loop suites, 28 `cmd_coro_e2e` tests) + the real C `tests/coroutine.test` through the VM (55/77) + a Node-executed wasm32 coroutine test; thread package via deterministic concurrency tests (`thread.test` is `testthread`-gated → N/A; no threaded oracle). |

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

**`catch` is yieldable.** `cmd_catch` no longer runs its body via `eval_source`
(a native re-entry that bumped `activation_depth`, so a `yield` inside `catch {…}`
errored `cannot yield: C stack busy`). It now compiles the body and parks a
`CatchReq` in `Vm.pending_catch`, which `dispatch_words` drains into a
`Tick::PushCatch` → a resumable *catch frame* (`Frame::new_catch`) on the explicit
stack — the same mechanism as `eval`/`uplevel 0`, but the frame **absorbs** the
body's completion instead of propagating it: `unwind` recognises the catch frame,
builds the body's errorInfo, and runs the shared `Vm::finish_catch` epilogue
(bind the result/options vars, deliver the status code as `catch`'s result — for
*any* completion code). `try` still re-enters natively (its `eval_body` calls
`eval_source`), so `yield` across `try` remains barriered.

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

**Sync primitives + thread pools.** `thread::mutex`/`rwmutex`/`cond` and
`tpool::*` share the same `Arc<Shared>` registries. A mutex is a
`Mutex<MutexState>` + `Condvar` (recursive: it honours a same-thread relock); a
rwmutex is an `RwState` + `Condvar` (many readers xor one writer); a cond variable
is a generation-counter gate + `Condvar`, and `thread::cond wait` releases its
paired mutex and re-acquires it on wake. `tpool::create`/`post`/`wait`/`get`/
`names` run a fixed worker set over a shared job queue — each worker builds its
own `Vm` from the `Send` compile-service factory, runs an optional `-initcmd`
once, then loops; `post -detached` discards the result and `get` blocks for (and
propagates the error of) a job. `forbid(unsafe)` is kept throughout: the
`Condvar`/`RwLock` primitives carry the safety, with no `unsafe impl Send`.

No oracle: the reference tclsh 9.0.4 is a **non-threaded** build (no `Thread`
package, `tcl_platform(threaded)` unset), so this subsystem is validated by
deterministic concurrency tests (`rust/tcl-vm/tests/cmd_thread_e2e.rs`, 25 tests:
sync/async send, per-worker isolation, worker-error propagation, exists/names/
release, an atomic 4-thread `tsv::incr` counter totalling 1000, the `tsv::*`
element operations, and the sync primitives — mutex roundtrip/recursion/serialised
read-modify-write, `cond` notify + timeout, rwmutex writer exclusion, and the
`tpool` post/collect/`-initcmd`-state/wait/error/`-detached`/names paths), with
semantics per the Tcl `Thread` package docs.

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

## Progress — VM as the primary wasm compile target

The VM is now the **default `tcl compwasm` backend**. Because the VM compiles Tcl
at run time and `ModuleAsm` is not serialisable (no serde in `tcl-bytecode`), the
artifact is a *generic* runner — the VM + compiler statically linked — that takes
a script at run time, not a per-script embedded module. The new crate
`rust/tcl-vm-wasm` (`crate-type = ["cdylib"]`, path deps `tcl-vm`/`tcl-compiler`/
`tcl-registry`/`tcl-bytecode`) builds to `wasm32-unknown-unknown` with **no
imports and no WASI** (pure-Rust `num-bigint`, so the tree-walker's
`WASI_SDK_PATH`/libtommath blocker does not apply) and exports a three-call
linear-memory ABI: `tcl_alloc(len) -> ptr`, `tcl_dealloc(ptr, len)`, and
`tcl_eval(ptr, len) -> packed(out_ptr, out_len)` — which builds a `Vm`,
`set_compiler`s the in-tree compile service, `eval_source`s the script (coroutines
included), and returns its captured output. It lowers with
`lower_to_ir_for_bytecode` (as `tcl-vm-cli` does), so the VM-faithful barriers fire
— a branching/nested `lmap`/`foreach` routes to its runtime builtin rather than an
inline shape the bytecode path can't compile. `make tcl-vm-wasm` builds and
self-checks it (`verify.mjs` runs a generator, the `set n [yield]` resume-value
idiom, a `yield`-across-`catch` case, and a `yield`-across-`lmap` case under Node,
asserting the tclsh-9.0.4 values) and ships `build/tcl-vm-wasm/vm.wasm`; the wasm
CI check `fmt`s and `clippy`s it for `wasm32`.

Surfaces select the backend with a `--backend vm|tree-walker` flag (default `vm`):
`tcl compwasm --backend vm` emits the shipped `vm.wasm` runner (compile-checking
the script first), `--backend tree-walker` keeps the legacy eval-fallback emitter
(the only one that yields a per-script WAT module, still used by the explorer's
WAT view); the MCP `compile_wasm` tool mirrors it, returning the bytecode
disassembly for `vm` and WAT for `tree-walker`. The product crate, its ABI, and
`verify.mjs` are pinned by `vm_wasm_crate_runs_coroutines_via_abi` in
`wasm_coro_e2e.rs`, and `compwasm_vm_backend_emits_runner` round-trips the CLI
default (both skip cleanly without the wasm toolchain).

## Progress — yieldable `lmap` (inline collecting loop)

A **straight-line** `lmap` now lowers to an inline *collecting* `foreach` on the
explicit stack, so `yield`/`yieldto` cross it (closing `coroutine.test`
9.2/10.1/10.3 — `lmap i {1 2} yield` / `{yieldto string cat}` inside an `apply`).
Previously every `lmap` barriered to the runtime `cmd_lmap`, which re-enters the
evaluator natively (`yield` → `cannot yield: C stack busy`) *and* the inline
`FOREACH_*` opcodes discarded the body result. The mechanism (mirroring `dict
map`'s keep-last-result trick, but with a VM-side accumulator so break/continue
stay sound):

- A new bare-byte `Op::LMAP_COLLECT` and a `foreach_collect` flag on
  `FOREACH_START`. `ForeachState` gains `collect` + `accum`; `FOREACH_START` reads
  the flag, `LMAP_COLLECT` pops the body result into `accum`, and `FOREACH_END`
  pushes `list(accum)` as the loop result (a plain `foreach` still yields `""`).
- Codegen strips the body's trailing `POP` and emits `LMAP_COLLECT` on the
  **fall-through path only**, and suppresses the loop-end `""` push for a
  collecting loop. A `break`/`continue` redirect jumps past `LMAP_COLLECT` (to
  `FOREACH_END`/`FOREACH_STEP`), so a skipped iteration contributes nothing — as C
  `lmap` does. The accumulator lives VM-side so it survives that redirect.
- Only a **straight-line** `lmap` lowers inline. A branching body (an
  `if`/`while`/`switch`/nested loop, or an unwinding `return`) compiles to a
  multi-block CFG the single fall-through collect point can't gather from, and a
  bare `break`/`continue` hits a pre-existing simple-`foreach` limitation (it jumps
  to the header, which re-runs `FOREACH_START`), so those — and `lmap` in a
  *consumed*/command-substitution position, which stays a runtime `INVOKE` — keep
  the runtime builtin (correct results, `yield` still barriered there). Oracle-
  checked in `cmd_collections_e2e.rs` (collection: empty/multivar/multilist) and
  `cmd_coro_e2e.rs` (yield-across-`lmap` generators), plus the wasm `verify.mjs`
  case.

## Progress — yieldable `subst` (scanner-driven subst frame)

`cmd_subst` used to run a native byte-scanner (`subst.rs`'s `subst_command`) that
evaluated each `[…]` via `vm.eval_source` — a native re-entry, so a `yield` in a
bracket errored `cannot yield: C stack busy`. It now parks the template + switches
in `Vm.pending_subst`, which the trampoline drains into a **subst activation
frame** — modelled on the `catch` frame, but *resumable* (re-entered once per
bracket). The frame (`Frame::subst`, an empty placeholder asm — it never runs
bytecode) is scanner-driven: `tick` routes it to `tick_subst`, which scans literal
/ backslash / `$…` runs **natively** into the accumulated output (these never
yield) and, on a top-level `[`, compiles the bracket body and pushes it as a
yieldable **child script frame**, leaving the cursor past the `]`. So a `[yield]`
inside a bracket freezes the whole scan (cursor + output) with the coroutine.

`unwind` folds each bracket's completion back by C's per-bracket subst rules
(`Vm::fold_subst_bracket`, subst-8.x/10.x): `Ok`/`Return`/any other code appends
the value and `continue` drops it (both **resume** the scan — the subst frame is
re-ticked), a `break` finalises with the output so far, and an error propagates.
The native `subst_command` stays for the non-command paths (`expr` command subst,
literal `PUSH` word substitution) and the `invoke_command` fallback runs the subst
frame via a nested drive (not yieldable there, as before). Closes `coroutine.test`
1.13/1.14 (`coroutine foo eval {subst {>>[yield a],[yield b]<<}}`). Oracle-checked
in `cmd_coro_e2e.rs` (yield/error across `subst`) and `builtins_e2e.rs`
(break/continue/return-per-bracket), plus the wasm `verify.mjs` case.

## Progress — C `coroutine.test` / `thread.test` harnesses

The actual C Tcl 9.0.4 `tests/coroutine.test` runs through the VM (bytecode) via
tcltest: **60 / 77 passing, 3 skipped** (the `testnrelevels`/`memory` constraints
gate C-only test commands the VM does not provide). `eval`, `uplevel 0`, a
straight-line `lmap`, and `subst` are now yieldable — a `yield`/`yieldto` reached
through them runs on the *explicit* activation stack (a transparent [script
frame](../rust/tcl-vm/src/exec.rs), the inline collecting loop, or the subst
frame, see below), closing 1.7/1.8/1.9/1.10/1.12, the lmap-in-`apply` cases
9.2/10.1/10.3, and the `subst`-as-coroutine cases 1.13/1.14. The 14 remaining
failures are the design divergence, not regressions:

- **`yield`/`yieldto` still cannot cross a host re-entry the VM runs on the
  native Rust stack** (3): `try` + event loop (7.14), and mutual `yieldto` whose
  `[yieldto …]` sits in a command-substitution argument *slot* that lowers to
  runtime `subst_word` (7.3, 12.1) — distinct from the `subst` command, which is
  now yieldable. (A *consumed*/branching `lmap` also still barriers to the runtime
  builtin, but `coroutine.test` does not exercise that form.)
- **Introspection/embedding features the VM does not implement** (11): `info
  frame` (3.2/3.6/3.7/10.9), child interpreters `interp create` (7.7/9.9/12.1),
  and detecting a coroutine whose namespace was deleted mid-suspend to raise
  `yieldto called in deleted namespace` (7.8–7.11).
- **A separate `coroinject` stacking-order divergence** (10.2 — stacked injects
  run in the reverse of C's order); unrelated to the yieldable surface.

**The yieldable-body mechanism.** A transparent *script frame*
(`Frame::new_script`, `is_script`) runs a compiled body on the explicit stack
instead of a nested native drive. On completion its result is delivered to the
parent exactly as an inline command's would be — an `ok` result is pushed to the
parent operand stack, a `break`/`continue` is offered to the parent's enclosing
loop, and on error it adds its `("eval"/"uplevel" body line N)` + `invoked from
within` frames (errorInfo parity verified against tclsh 9.0.4). `EVAL_STK` pushes
one directly; the `eval`/`uplevel 0` builtins defer their body via a
`Vm.pending_eval` slot that `dispatch_words` drains into a `Tick::PushScript`
(mirroring how `coro.pending` becomes `Tick::Suspend`).

The C `thread.test` (52 tests) is **not applicable** to the VM's thread package:
every test is gated on the `testthread`/`thread` constraints, which require the
core `tcl::test` `testthread` command and a `Thread`-package/threaded build — a
different design from the VM's shared-nothing `Thread`-extension-style package
(Phase 3). Run through the VM, all 52 skip. The VM's threading is instead
validated by the native `cmd_thread_e2e.rs` suite (per the `Thread` package docs;
the reference tclsh is non-threaded, so there is no threaded oracle).

**Remaining (still Open):**
- **`eval`, `uplevel 0`, `catch`, a straight-line `lmap`, and `subst` are now
  yieldable** (they run their body on the explicit stack via a transparent
  script/catch frame, the inline collecting loop, or the scanner-driven subst
  frame — see the harness section). Still on the **native Rust stack**, so a
  `yield` across them errors `cannot yield: C stack busy`: `lmap` in a *consumed*/
  branching position (which stays on the runtime `cmd_lmap`), `try` (its
  `eval_body` re-enters via `eval_source`), `apply` in an *arbitrary* position,
  `lsort -command`, and a `[yieldto …]` command substitution in an argument *slot*
  that lowers to runtime `subst_word` (distinct from the now-yieldable `subst`
  command). `yieldto`, the creating-namespace command resolution, and `info
  level`/`info coroutine` are done.
- Introspection/embedding gaps behind the other ~11 `coroutine.test` failures:
  `info frame`, child interpreters (`interp create`), and raising `yieldto
  called in deleted namespace` when a coroutine's namespace is deleted while it
  is suspended.
- The `tcl-registry` `Thread`-package `CommandSpec`s for the sync primitives are
  LSP metadata only (the runtime and the `RUST_ISSUE_006` core-backing gate do
  not require them). The `thread` package (including the `mutex`/`cond`/`rwmutex`/
  `tpool` primitives now landed) is a native-only VM feature — it needs OS
  threads; on wasm the VM runs single-threaded with coroutines, matching the
  plan's per-backend model.

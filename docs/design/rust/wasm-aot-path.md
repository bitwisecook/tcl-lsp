# RT-WASM path forward — native WASM + AOT with real Tcl frames

> **Historical proposal — reference design.** This was written against the
> `main`-branch **Python + Zig** stack, when the plan was to reuse the Zig WASM
> runtime as-is and port only the emitter. That runtime has since been **ported
> to Rust** (`runtime/rust/`, the `tcl-runtime` crate — see
> [`rust-runtime-port.md`](../runtime/rust-runtime-port.md)), and the Python
> emitter is now the Rust `tcl-compiler`. Read the "Zig runtime" references below
> as the historical **reference design** the Rust port targets; the ABI, frame
> model, and difficulty register still apply, now realised in Rust rather than
> Zig.

A proposal for taking the Rust **RT-WASM** track past the current eval-fallback
tier to a genuine ahead-of-time (AOT) native WASM compiler — the architecture the
**Python + Zig** stack on `main` already proves out. Treat the "tcl frames but
native WASM and AOT" shape as the **target**, with eyes open about where the
difficulty actually lives.

This document is grounded in two reads of the existing implementation:
the Python emitter (`compiler/codegen/wasm/`, ~20 K LOC) and the Zig runtime
(`runtime/zig/`, ~83 K LOC). Both are mature and battle-tested; the proposal is to
*reuse one and port the other*, not to reinvent either.

## What `main` already achieved (the reference design)

The Python+Zig WASM stack is **not** an eval-fallback shim — it is a full native
AOT Tcl compiler split into two cooperating halves.

### 1. The Zig runtime is a complete, reusable WASM artifact

`runtime/zig/` compiles to `tcl_runtime.wasm` (a WASI **reactor** module:
`entry = .disabled`, `_initialize` runs libc constructors). It **exports ~100
`tcl_*` functions** — the native ABI the compiled program calls — across value
lifecycle, float-aware arithmetic + 20-odd math fns, list/dict/string/array ops,
the **frame stack** (`frame_push`/`frame_pop`, `local_get`/`local_set`,
`frame_local_at` indexed fast-path, `frame_alias_global`/`frame_alias_named`/
`frame_alias_frame_var`, `upvar_resolve_depth`/`upvar_walk_relative`), exception
+ control-flow signal flags (`catch_enter`/`leave`, `flow_check_return`/
`flow_take_return`/`flow_consume_break`), proc registry, namespaces, and I/O.
The `TclObj` value model (`valtypes/tcl_obj.zig`) is a 32-byte struct with
refcounting, shimmering (string ⇄ int/float/list/dict/bignum), inline short
strings, and **tagged immediates** (`(v<<1)|1` for `0 ≤ v < 2³⁰`, no allocation).
Coverage is ~80 % of Tcl 8.6 / ~85 % of 9.0.3, plus a `tcl_runtime_with_tcltest`
variant. It is pinned to C Tcl 9.0.3 (§0).

**Key consequence:** the runtime is *already done* and is explicitly retained by
the rewrite plan ("the Zig WASM runtime stays as the out-of-process runtime for
compiled scripts"). The Rust side does **not** re-port 83 K lines of runtime — it
targets the runtime's existing exported ABI.

### 2. The Python emitter compiles Tcl → native WASM against that ABI

`compiler/codegen/wasm/api.py` runs an 8-phase pipeline (var-escape →
optional inline/LICM/DCE/GVN → import scan → proc-index → emit `::top` → emit
each proc). The emitter (`_emitter/`, a mixin family with ~39 per-command files)
produces **real instructions**:

- **Values** are i32 handles (tagged immediates or heap pointers) with a
  compile-time **OWNED / BORROWED** ownership discipline (`_ownership.py`);
  owned slot-writes are wrapped with conditional `tcl_obj_retain`/`release` so
  the runtime's refcounts stay balanced without a frame.
- **Tcl frames** are runtime-heap structures, but proc locals are **mirrored
  into WASM `local`s**, and — driven by **var-escape analysis** — frames are
  **elided entirely** (`wants_frame == false`) for procs whose locals never
  escape via `upvar`/`global`/`info locals`. Frame-present procs sync slots ⇄
  frame around dynamic barriers (`_emit_interp_boundary` / `_emit_frame_readback`).
  `global`/`variable`/`upvar` become `frame_alias_*` registrations.
- **Each proc is a WASM function** `(i32…) -> i32` (TclObj params/result), with a
  prologue (register-compiled, push frame, build `info level 0` argv, default
  substitution, alias setup) and epilogue (error-frame stamp, frame pop, ns
  restore). Calls are direct `call $idx` between compiled procs, or the
  `env.call_compiled_proc` host bridge from the interpreter; `return`/error
  propagate via the runtime's signal flags.
- **`expr`** lowers its pre-parsed AST to inline integer ops where it can
  (`i64.add` then re-box), falling back to the float-aware `tcl_arith_*` helpers.

### 3. Linking is Binaryen `wasm-merge`

`link.py` statically inlines `source`d files into one IR/CFG; `_bundle.py` runs
`wasm-merge` to combine `tcl_runtime.wasm` (module `"tcl"`) with the user module
(its `import "tcl" "*"` calls resolve against the runtime's exports) into a single
deployable `.wasm` over one shared linear memory. Only WASI + the optional
`env.call_compiled_proc` bridge remain as imports. `wasm-opt --asyncify` is the
Stage-2 path for coroutines.

## Where the Rust port stands today

- **eval-fallback emitter** (`tcl-compiler::codegen::wasm`): boxes each leaf
  command as a `tcl_eval` string; control flow is real structured WASM
  (`if`/`block`/`loop`/`br` with `tcl_expr_bool` conditions). The 4-function ABI
  (`tcl_obj_new_string`/`tcl_eval`/`tcl_obj_release`/`tcl_expr_bool`) is the
  `runtime/rust/src/codegen_abi.rs` surface.
- **Already ported and reusable by the AOT emitter:** the IR/CFG/SSA, the
  **var-escape analysis** (FE-VARESCAPE — the exact soundness predicate frame
  elision needs), the inliner (FE-OPT, incl. the v3-simple parameterised shape),
  and the optimiser passes the Python Phase-0.x steps call.
- **Verification scaffold (new):** `tcl-fuzz`'s `wasm-diff` arm embeds wasmtime
  and drives the eval-fallback module's control flow with a `tcl-vm`-backed host,
  asserting output parity against direct `tcl-vm`. This already validates the
  **control-flow** codegen; it is the seed of the native differential (below).

## Proposed path — staged, each stage independently shippable

The guiding principle mirrors the Python design's own fallback discipline: the
**eval-fallback tier stays as the correctness safety-valve**, and native
per-command emitters override it incrementally (the Python emitter hooks already
`return False` to fall back). No big-bang.

### Stage A — ABI contract + runtime in CI *(foundation, do first)*
Pin the ~100-function `tcl_*` ABI as a **single Rust-side contract** (signatures
+ ownership notes), ideally **generated from the Zig `export fn` set** so it
cannot silently drift. Wire `runtime/zig` into the build to produce
`tcl_runtime.wasm` as a CI artifact (Zig 0.16 + Binaryen are already in the
session toolchain). Deliverable: a `tcl-wasm-abi` module the emitter imports
against, plus an **ABI-conformance test** that fails if the Zig exports and the
Rust contract disagree. *This is the single most load-bearing risk control.*

### Stage B — native value + expr + leaf commands
Port the value model (tagged-immediate/handle, `obj_new_int/string/float`,
`obj_get_*`) and the **OWNED/BORROWED** ownership tracker (`_ownership.py`),
then the `expr`-AST → native-instruction lowering and the highest-frequency leaf
emitters (`set`/`incr`/`list`/`lindex`/`string`/`dict`/`puts`). Each command's
native emitter replaces its eval-fallback box; unhandled commands keep boxing.
Verify per-command against the **linked Zig runtime** (Stage F harness).

### Stage C — frames *(the "tcl frames" core — the hard part)*
Port frame push/pop, the WASM-local mirror, **escape-driven frame elision** (the
FE-VARESCAPE predicate is already in Rust), the slot⇄frame sync points around
dynamic barriers, and the `frame_alias_*` lowering for `global`/`variable`/
`upvar`. This is where soundness is subtlest (see Difficulties).

### Stage D — procs / AOT calls
Each Tcl proc → a WASM function; emit the prologue/epilogue (register-compiled,
argv build, default substitution, alias setup, error-frame stamp); direct
`call $idx` between compiled procs + the `call_compiled_proc` bridge; wire the
`flow_check_return`/error-flag propagation after each call.

### Stage E — linking / bundling
Port `link.py` (static `source` inlining over the IR) and `_bundle.py`
(`wasm-merge` against `tcl_runtime.wasm`, DWARF strip, runtime-variant
selection) as the `tcl-wasm` CLI `--link` path. Output: a single deployable
`.wasm`. (`IRInterpBoundary` IR node + the WASM-side DCE/GVN passes fold in here.)

### Stage F — the real value-differential arm *(verification, upgrades the seed)*
Replace the `tcl-vm`-backed `wasm-diff` host with the **actual linked artifact**:
compile a program → `wasm-merge` with `tcl_runtime.wasm` → run under
wasmtime-WASI → capture stdout → diff against `tclsh9.0` (§0 ground truth). This
is the genuine third differential arm the TOOL-FUZZ residual calls for, and it
graduates the current control-flow-only check to full value/side-effect parity.
The harness already exists in `tcl-fuzz`; only the host backing changes.

## Difficulties (honest accounting — this is not a free lunch)

1. **ABI drift is the top risk.** The Rust emitter and the Zig runtime must agree
   on ~100 signatures *and* their ownership semantics (who frees what, `rc 0` vs
   `+1` returns). A mismatch is a silent leak or use-after-free in the runtime,
   not a compile error. *Mitigation:* generate the contract from the Zig exports;
   gate on an ABI-conformance test; lean on the Zig runtime's `leak_check`.

2. **Frame elision soundness.** Eliding a frame is only sound when locals truly
   never escape. FE-VARESCAPE is ported, but the WASM consumer stresses it in
   ways the analyser hasn't been driven before (e.g. an `uplevel` reaching a
   nominally-elided caller). A wrong elision is a miscompile. *Mitigation:* the
   Stage-F per-command/per-proc differential, and conservative default-to-frame.

3. **Ownership/refcount discipline.** The OWNED/BORROWED tracker is the most
   error-prone piece of the emitter; getting a retain/release wrong leaks or
   double-frees. *Mitigation:* port it early (Stage B), test under the runtime's
   leak counters, and keep the eval-fallback path as an oracle.

4. **Two runtimes, one semantics.** The AOT targets the **Zig** runtime; the Rust
   tree-walking `runtime/rust` and the `tcl-vm` are separate. All three must stay
   pinned to C Tcl 9.0.3 or differentials produce false findings. The `tcl-vm`
   value-diff seed is fine for control flow but **cannot** be the oracle for the
   native arm — Stage F must use the Zig runtime + tclsh.

5. **Scale.** ~20 K LOC of emitter to port. It is bounded and **parallelises**
   (per-command emitters are independent, and each can land behind the
   eval-fallback safety-valve), but it is a multi-PR effort, not a sprint.

6. **Toolchain in the loop.** Zig (build the runtime) + Binaryen (`wasm-merge`/
   `wasm-opt`) become build/CI dependencies. Both are already provisioned in the
   session start hook; they must be made first-class in `make`/CI.

7. **Coroutines/events stay Stage 2.** `after`/`vwait`/`coroutine`/`yield` need
   `asyncify` (≈2× overhead, ≈1.5× size) and the runtime's Stage-2 paths — defer
   until the synchronous core is solid.

## Why this is the right shape

- It **reuses the 83 K-line Zig runtime as-is** — the bulk of the work is already
  done and explicitly meant to survive the rewrite.
- It delivers exactly "tcl frames + native WASM + AOT," reusing the analyses the
  Rust side already has (IR/CFG/SSA, var-escape, inliner, optimiser).
- It is **incremental and always-shippable**: the eval-fallback tier is the
  safety net; native emitters override per command; nothing regresses.
- It closes the open RT-WASM residuals in dependency order: native emitter
  (Stages B–D), `IRInterpBoundary`/DCE/GVN + `--link` (Stage E), and — for the
  consumers — TOOL-FUZZ's value differential (Stage F), TOOL-EXPLORER's rich
  `to_explorer_json` densifying as real instructions appear, and FE-OPT v3's
  capture-sensitive rewriter gaining its execution-differential consumer.

## Sequencing against the existing plan

Stage A unblocks everything. Stages B→C→D are the emitter core (B and the leaf
emitters can proceed in parallel once the value model lands; C is the gating
hard part). Stage E (linking) depends on a working emitter. Stage F can run
against partial emitters from Stage B onward (boxed commands still execute via
the runtime's `tcl_eval`), so verification grows with the emitter rather than
waiting for the end.

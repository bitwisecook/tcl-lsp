# Runtime & execution — port gaps (WASM / bytecode VM / runtime)

> **Audience:** Maintainer / Contributor
> **Type:** Design (status audit — consolidated index)
> **Scope:** the runtime & execution layers only. The front-end / analyser /
> server / API / tooling gaps live in
> [`python-rust-port-gaps.md`](../rust/python-rust-port-gaps.md); this doc was
> split out of that audit's "Stage 2" so the runtime work can be enumerated,
> honestly accounted for, and finished on its own track.

This is the single entry point for the three runtime & execution subsystems that
are **not yet at parity** with C Tcl 9.0.3 / the Python oracle:

1. **RT-WASM** — WASM codegen emitter + `tcl-wasm` bundling.
2. **RT-VM** — the bytecode VM (`rust/tcl-vm`).
3. **`runtime/rust`** — the tree-walking runtime port.

Where a live, regenerable tracker exists it is the **authoritative** source (this
index does not duplicate its per-row detail, to avoid staleness):

- **VM opcode coverage:** [`tclvm-opcode-status.md`](tclvm-opcode-status.md) —
  the 191-instruction binary-compat checklist.
- **VM tcltest parity:** [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md) —
  per-stem `passed/skipped/failed` vs C Tcl 9, regenerate with
  `cargo xtask tcltest-sweep --backend both`.

**Tiered delivery plan** — how the VMs and the runtime are brought to C-Tcl
parity, bottom-up. These are the *plan* documents (the trackers above are the
*scoreboard*):

- **The capability ladder:** [`tcl-test-tiers.md`](tcl-test-tiers.md) — the
  semantic tier grouping (which upstream `.test` files belong to each tier and
  why a lower tier gates the ones above it). The delivery order: bring the
  runtime to C parity one tier at a time, bottom-up, and lock each green tier
  so later work can't silently regress it.

Status legend: ✅ done · 🟢 done bar listed residuals · 🟡 partial · 🔴 not started.

## Headline (do not hand-edit the counts — read them off the trackers)

| Subsystem | Status | Headline |
|---|---|---|
| RT-WASM — WASM emitter | 🟡 | the per-command emitter package is the **largest single gap** |
| RT-VM — opcodes | 🟡 | see the live count in [`tclvm-opcode-status.md`](tclvm-opcode-status.md) |
| RT-VM — tcltest parity | 🟡 | see the live tally in [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md) |
| runtime/rust — tree-walking port | 🟡 | most subsystems partial |

Earlier revisions of this file duplicated the tracker numbers here and they went
stale within weeks. The trackers are regenerable; this index is not. Follow the
links.

**Cross-cutting reality:** TclOO and coroutines are implemented in **both**
`runtime/rust` (`cmd_oo.rs`, `cmd_coro.rs`) and the bytecode VM
(`rust/tcl-vm/src/cmd_oo.rs`, `cmd_coro.rs`) — `oo::class create` + method
dispatch and `coroutine`/`yield`/`yieldto` all run under `tclvm` today. The
residual is in the WASM **emitter**, not the VM, plus the coroutine re-entry
barriers listed in §4.

---

## 1. RT-WASM — WASM codegen + runtime 🟡 (largest single gap)

Owns `tcl-compiler::codegen::wasm`, `runtime/rust`, and a new `tcl-wasm` bin.
The eval-fallback emitter and `tcl compwasm` wiring have landed (binary/WAT
output, `wasmtime`-validated); leaf commands now propagate their **completion
code** (`tcl_eval_code` + the emitter's completion dispatch),
so `error`/`return` unwind and a `break`/`continue` re-enters the compiled loop. **Scale of the gap:** the Rust emitter is ~1.5 K
LOC across 4 files (`codegen/wasm/{backend,encoding,ir,mod}.rs`); the Python
package it must reach parity with is **~20.6 K LOC across 49 modules**
(`compiler/codegen/wasm/`), including a per-command emitter for each of
`append`, `apply`, `array`, `catch`, `clock`, `concat`, `dict`, `error`,
`fconfigure`, `fcopy`, `format`, `info`, `lappend`, `linsert`, `list`,
`lreplace`, `lsearch`, `lsort`, `puts`, `regexp`, `return`, `runtime`, `scope`,
`set`, `string`, and `uplevel`.

**Remaining:**

- **(large)** Finish the WASM emitter (`wasm_codegen_module`) — only the Phase-1
  IR + encoding is ported vs the ~13-module Python emitter package.
- The `IRInterpBoundary` IR node + its insert pass.
- The IR-rewriting `passes/dce.py` and `passes/gvn.py`.
- `source_inliner` / `stdlib_prelude` (WASM-bundle self-containment).
- The `tcl-wasm` CLI + `--link` (Binaryen) bundling — standalone self-contained
  module.
- **No bytecode-VM fallback:** a construct the WASM emitter cannot lower is an
  error, not a fall-back to the bytecode VM.
- **Real-runtime link not in CI:** `wasm_real_link.rs` validates only trivial
  snippets; linking the emitted module against the real `runtime/rust` wasm (and
  a C extension) end-to-end is not yet exercised.

**Downstream (WASM-gated) tooling — also blocked here:**

- **TOOL-EXPLORER** — the per-instruction web-GUI (`to_explorer_json` →
  `tcl_explorer::wasm_explorer`) has landed; it **densifies automatically as
  RT-WASM emits real instructions**, no separate work beyond the dependency.
- **TOOL-FUZZ** — the differential fuzzer has landed; the residual is re-backing
  the `wasm-diff` arm with the **real linked runtime** for a full value
  differential (gated on RT-WASM).

---

## 2. RT-VM — bytecode VM 🟡

Owns `tcl-vm` (+ the `tcl-vm-cli` / `tclvm` driver). The engine core is solid
(loads the real Tcl 9 `tcltest.tcl` end-to-end) and the VM-vs-tclsh differential
`bug_*` cmd-tests are all closed (math/`expr`, `string`/`format`,
dict/array/`lsort`, `try`/control, `namespace which -variable`).

### 2a. Opcode coverage (live count in `tclvm-opcode-status.md`)

- **enum-only** (`[~]` — emittable but not executed): the highest-leverage
  bucket. `{*}` expansion (`expandStart`, `expandStkTop`, `invokeExpanded`),
  `tailcall`, and several string ops (`strmap`, `strclass`, `strreplace`,
  `numericType`). The dict family (`dictGet`, `dictSet`, `dictUnset`,
  `dictIncrImm`, `dictAppend`, `dictLappend`, `dictExists`),
  `upvar`/`nsupvar`/`variable`, `lsetList`/`lsetFlat`, and the exception-range
  markers (`beginCatch4`, `endCatch`, `pushResult`, `pushReturnOpts`) are now
  dispatched in `exec.rs`; the exact live count is auto-recomputed in
  `tclvm-opcode-status.md`.
- **54 missing** (`[ ]` — not yet in the `tcl-bytecode` `Op` enum): the OO family
  (`tclooSelf`/`tclooClass`/`tclooNext`/…), coroutine (`yield`, `coroName`,
  `yieldToInvoke`), dict iteration (`dictFirst`, `dictNext`,
  `dictUpdateStart`/`End`, `dictExpand`, `dictRecombine*`), stack-variant
  loads/stores/incrs (`loadScalarStk`, `storeScalarStk`, `incrArray*`), the
  append/lappend array families, `unset*`, and introspection
  (`currentNamespace`, `infoLevelNumber`/`Args`, `resolveCmd`).

The `[~]`→`[x]` transitions (exception, dict, `{*}`) are the ones the checks/
optimiser already assume; the `[ ]`→enum→exec work for OO/coroutine is a whole
subsystem (see §4).

### 2b. tcltest parity (live tally in `rust-vm-tier-parity.md`)

`CRASH` = an uncaught error/timeout aborts a whole file (highest leverage — one
fix unlocks it). The structural gaps behind the scoreboard:

- **(P1) Uncaught-error abort / error-propagation.** A test-body error that
  escapes tcltest's `catch` propagates to the module top and aborts the whole
  `run_test` driver (e.g. `info.test` halts at info-8.3, `proc.test` on a bare
  `VM error:`). An `uplevel`/`catch`/error-propagation gap (not a deadlock);
  it zeroes the remainder of those suites. (Re-baseline the harness in
  `--release` first — debug slowness × the timeout masquerades as a hang.)
- `namespace` / `var` / `upvar` **depth** (~290 failures combined) — namespace
  canonicalisation of multiple/trailing `::` runs; deeper variable-scoping /
  introspection. The three share the model.
- `error.test` (29 failing) — 22 are `[try]`-coverage tests needing the
  **`-level` countdown** (only `-level 0` is immediate today); the rest are
  `info errorstack` / the `-errorstack` option (unimplemented) and errorInfo
  edge cases.
- Smaller per-suite gaps — `switch` (14), `for` (8), `foreach` (6), `incr` (3),
  `if`/`set` (2), `while` (1).

### 2c. Structural / command-surface gaps

- **Structural** — give the bytecode backend real exception ranges (`beginCatch`)
  and a fixed nested-complex-`foreach` / `lmap`-collecting codegen, then drop the
  `for_bytecode` barriers (`try` / nested-`foreach` / `lmap` run via runtime
  builtins today — correct but not inline).
- **Missing command surface** — TclOO, `clock`, `after` / `vwait`, coroutines,
  real I/O (`open` / `gets` / `seek`), and `info functions` have all landed
  since this section was first written; re-probe before quoting it. What is
  still missing is `info cmdcount` / `info frame` / `info hostname` and residual
  `file` / `interp` / `namespace` subcommands. (`info cmdcount` cannot reach
  exact-count parity without per-bytecode command counting — an "exists but
  approximate" subcommand.)

---

## 3. `runtime/rust` — tree-walking runtime port 🟡 (off-workspace)

`runtime/rust/` is the standalone Rust WASM runtime,
kept out of `cargo test --workspace` (it needs raw-pointer
`unsafe` over shared linear memory) and gated via `make runtime-rust-test`. It is
the eventual in-process tree-walking interpreter and the wasm32 runtime the
emitted modules link against. Most subsystems are **partial**:

- **`TclObj` + refcount / shimmer** (T1.1) — partial; round-trips leak-clean, but
  the full typed-rep machinery is incomplete.
- **valtypes** — list, dict, and string capacity/char-ops landed; **array, arith,
  format, encoding, hash_table, bs, chars, arena, parse_cache** still to port
  (each with a representation-decision note).
- **bignum** — `obj` carries `i64` today; the **libtommath-style arbitrary
  precision tower** (never-wraps integer arithmetic) is an open representation
  decision, `cfg`-gated off behind `have_tommath` on wasm32.
- **parse / subst** (T1.2) — partial; unit parity only.
- **interp eval loop** (T1.4) — parse→subst→dispatch + `{*}` landed; full
  control-flow / proc depth follows.
- **frames / namespaces / procs** — partial; `namespace delete`, `rand`/`srand`
  RNG state, and deeper proc/catch handling remain.
- **dispatch** — partial; resolves through the namespace tree, builtin surface
  fills in incrementally.
- **builtins (`cmds/`)** — a large surface runs (drives the unmodified Tcl 9
  library to `package require tcltest`), but the tcltest sweep is incomplete
  (e.g. `dict.test` 272/373, `lrange` 1759/1766); per-command parity ongoing.
- **regex** — the pure-Rust `tcl-regex` ARE engine drives `regexp`/`regsub`.
  The former **C** Henry-Spencer engine (`build.rs` + FFI shim, `have_regex`)
  has been removed, so regex now builds on `wasm32` too; nothing vendors or
  fetches the C sources. C consumers still link the engine — through the
  pure-Rust `regex_capi` C-ABI shim (`TclReComp`/`TclReExec`/…), not C sources.
- **OO / coroutines** — `cmd_oo.rs` (TclOO: classes, super-classes, `oo::define`,
  method dispatch) and `cmd_coro.rs` are implemented here; native stack-swap +
  threads are `cfg`-gated off under the `BrowserHost` capability host on wasm32.
- **lib root / ABI** — compiles and links for `wasm32-unknown-unknown` and an
  emitted module runs against it end-to-end, but the **rest of the
  `tcl_*`/`obj_*` symbol set** and **true PIC** (drop the per-program reserved
  constant-pool gap for `__memory_base`) remain.

---

## 4. Cross-cutting subsystems (span VM + codegen + runtime)

These are whole features, not single opcodes, and each must land in **three**
places (VM exec, bytecode codegen, and — where not already present — the runtime):

- **TclOO** — present in `runtime/rust` **and** in `tcl-vm`
  (`rust/tcl-vm/src/cmd_oo.rs`); `oo::class create` + method dispatch run under
  `tclvm`. The residual is the WASM `codegen` OO emitter.
- **Coroutines** — present in `runtime/rust` **and** in `tcl-vm`
  (`rust/tcl-vm/src/cmd_coro.rs`), built on the explicit-stack trampoline rather
  than opcodes. Residual: `yield` still cannot cross `try`, `apply`, or a
  value-consumed `lmap` (issue #1311), and the WASM `codegen` emitter.
- **Exception model** — `beginCatch4`/`endCatch` are enum-only in the VM; wiring
  them (plus real exception ranges in bytecode codegen) removes the `for_bytecode`
  `try`/`catch` barriers and is the root of the (P1) error-propagation aborts.
- **Dict iteration** — `dictFirst`/`dictNext`/`dictUpdate*` are missing opcodes;
  needed before `dict for` / `dict update` / `dict with` compile inline.

---

## How to finish this track (leverage order)

1. **Exception model** (`beginCatch`/`endCatch` exec + bytecode exception ranges)
   — unblocks the (P1) aborts that zero whole tcltest suites.
2. **`{*}` expansion + dict ops** (`expandStart`/`invokeExpanded`, `dictGet`/`Set`
   /iteration) — flips a cluster of enum-only opcodes to executed.
3. **The `namespace`/`var`/`upvar` depth model** (~290 failures, shared model).
4. **Coroutine re-entry barriers** (issue #1311) — `try` / `apply` /
   value-consumed `lmap` are the last three native re-entries in the VM.
5. **RT-WASM emitter** — the ~13-module per-command emitter package (including
   the OO and coroutine emitters) + `tcl-wasm` `--link` bundling, then re-back
   TOOL-FUZZ's `wasm-diff` with the real link (issue #1313).

Regenerate the trackers (`tclvm-opcode-status.md` counts,
`cargo xtask tcltest-sweep --backend both`) after each landing to keep the
headline honest.

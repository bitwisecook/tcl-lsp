# Rust runtime port — productionising C-Tcl-extension-to-WASM

Status: **bootstrapping.** The end-to-end mechanism (compile an unmodified C
Tcl extension to WASM and link it against our runtime + compiled user code,
API-not-ABI) is proven by the three throwaway spikes under
[`runtime/rust-spike/`](../../../runtime/rust-spike/README.md). The durable
contract is [`c-extension-abi.md`](c-extension-abi.md). This document is the
**source of truth** for turning that proof into a shipped capability, and it
must be kept current **every PR**.

> Modelled on [`docs/rust-rewrite.md`](../../rust-rewrite.md) — same SYNC-* /
> GAP-AUDIT-* sync discipline, same "component status + gate" tracking. Where
> `rust-rewrite.md` ports the *compiler/LSP* Python tree to Rust, this document
> ports the *WASM runtime* (`runtime/zig/`) to Rust and links C extensions into
> the AOT-first whole-program artifact. The two efforts share the `rust`
> branch's crate workspace but have disjoint component tables.

## North star

AOT-first, one artifact. The WASM AOT compiler
(`core/compiler/codegen/wasm/`) compiles as much Tcl as it can prove ahead of
time; the Rust-ported runtime (`runtime/rust/`) is the **support library +
interpreter fallback** for what can't be proven AOT-safe; C Tcl extensions are
linked in; and everything links into **one WASM+WASI artifact**.

The end state, stated as testable claims:

1. A program not heavily reliant on metaprogramming (no `eval`/`uplevel`/
   dynamic command or variable names) compiles **entirely AOT** and never
   enters the interpreter.
2. Common metaprogramming patterns are AOT-compiled by heuristics with a clean
   fall-through to runtime interpretation (a new staircase stage beyond S6 —
   see [Track 3](#track-3--aot-first-execution--whole-program-link)).
3. C Tcl extensions up to **sqlite3/tclsqlite** load and run, against the Rust
   runtime, via the production loader, with the surrounding script AOT-compiled.
4. The **C Tcl 9 test suite** (`tmp/tcl9.0.3/tests/*.test`) is the correctness
   gold standard; no file passing on the Zig baseline regresses.

The Zig runtime (`runtime/zig/`) stays the behavioural **oracle** until Rust
reaches parity. Do **not** regress the compiler/LSP or the Zig runtime.

## Reference implementations (use both freely)

- **Canonical C Tcl 9 source** — `tmp/tcl9.0.3/generic/*.c`
  (`tclBasic.c`, `tclExecute.c`, `tclObj.c`, `tclParse.c`, `tclUtil.c`,
  `tclCmdIL.c` / `tclCmd*.c`, `tclInterp.c`, `tclIO.c`, `tclOO.c`,
  `tclTomMath/*`, …). First-class information source for exact semantics, edge
  cases, the C API, and error/refcount behaviour.
- **`runtime/zig/`** — the current port being mirrored *and* the behavioural
  oracle (parity gate, tcltest sweep, leak baseline).

Where the two differ or Zig is incomplete, **defer to the C source + the Tcl 9
test suite** as ground truth.

## Read first (build on these; do not re-derive)

| Doc | What it gives you |
|---|---|
| [`c-extension-abi.md`](c-extension-abi.md) | ABI (§4), link models (§5), measured GOT findings (§11), scoped next steps (§13) |
| [`runtime/rust-spike/README.md`](../../../runtime/rust-spike/README.md) | The three throwaway spikes — reimplement properly, do not derive shape from |
| [`memory-management.md`](memory-management.md) + [`refcount-contract.md`](refcount-contract.md) | TclObj model + refcount discipline (cross-check vs `tclObj.c`) |
| [`../compiler/wasm-aot-staircase.md`](../compiler/wasm-aot-staircase.md) (+ s0..s6) | AOT north star + staircase; the metaprog heuristics extend this |
| [`zig-runtime-roadmap.md`](zig-runtime-roadmap.md) | The Zig runtime's own roadmap and layering |
| [`../../../AGENTS.md`](../../../AGENTS.md) | Zig runtime layering, the WASM parity gate (`make check-wasm-parity`), workflow |
| [`../../rust-rewrite.md`](../../rust-rewrite.md) + `docs/design/rust/` | The Rust migration this fits into |

Upstream trees: `tmp/tcl9.0.3/{generic/{tcl.h,tclDecls.h},doc/*.3,tests/*.test}`;
dltest samples in `tmp/tcl9.0.3/unix/dltest/`; tcllib at `tmp/tcllib-2.0`.

## Branch + base

Branched off `rust` (the spikes + design doc are merged there). Many small,
individually-gated PRs. Branch anchor: `rust`@`8150eca` (#549 — the spike
merge). The Zig sync log below anchors against this same commit (the state of
`runtime/zig/` at branch point).

---

## Target — WASM + WASI (chosen target + rationale)

**Decision.** Two surfaces, deliberately split:

| Surface | Target | Why |
|---|---|---|
| **Extension-loading path** (side modules: compiled user code + C extensions) | **core wasm + shared linear memory + a growable, exported `__indirect_function_table`** | The component model is *shared-nothing*; it fights the shared-linear-memory C-extension ABI (`c-extension-abi.md` §3–§5). Dynamic linking needs `__memory_base`/`__table_base` allocated from one shared memory and `call_indirect` across modules through one shared table. This stays on **core wasm** regardless of how the outer runtime ships. |
| **Outer host/runtime interface** (clock, filesystem, channels, stdio) | **WASI preview 1** today (`wasm32-wasip1`); **evaluate preview 2/3** as the host interface only | Preview 1 is what the shared-memory dynamic-linking model supports today. Preview 2/3 (the component model) may wrap the *outer* artifact later, but must not push the extension path off core wasm. |

Rust targets in use: `wasm32-wasip1` (runtime + host), `wasm32-unknown-unknown`
(side modules where no WASI is needed). Newer wasip targets are added as the
shared-memory model adopts them. Record any target change here with rationale.

**Linker flags (from `c-extension-abi.md` §8 / §5.2):**

- Runtime/main module: `--export-table --growable-table` + exported `memory`.
- Side modules: `-fPIC` → `wasm-ld --experimental-pic -shared --no-entry
  --import-memory --import-table`.

**Toolchain (pre-installed):** stable Rust (`wasm32-wasip1` +
`wasm32-unknown-unknown`); `zig` (use `zig cc` for C — bundled wasi-libc);
`wasm-ld`; `wasmtime`.

---

## Chunking strategy

Three interleaved tracks, sequenced by what each tier/north-star step needs.
Each chunk is one PR (or a short PR series), scoped and gated; never merge a
tier or stage without its gate green. If a needed surface is large (channels
for Memchan; eval-loop depth for sqlite's `db eval`), it lands as its own gated
PR **before** the gate that needs it.

- **Track 1 — port the runtime to Rust** (`runtime/rust/`): real TclObj +
  refcount, then parse/subst → eval loop → frames → namespaces → command table
  → builtins, mirroring `runtime/zig/` with C source for semantics. Re-export
  the `tcl_*`/`obj_*` symbols AOT codegen imports so parity stays green.
- **Track 2 — production C dynamic-linking interface**: promote the spike
  headers to shipped headers backed by real impls; land the C-API
  ownership/error contract first (a gate rejects un-annotated exports); move the
  loader from the Python spike into the runtime/host; add the
  external-command-registration dispatch entry.
- **Track 3 — AOT-first execution + whole-program link**: factor per-command
  lowering behind a **backend-agnostic emit protocol/trait + command-emission
  registry** (one source of truth targeting tclvm / wasm / llvm-ir); make AOT
  the primary path; extend `wasm_link.py` to link extension objects (static
  Model A where possible, dynamic Model B otherwise); drive AOT coverage up the
  staircase; add the new metaprogramming-heuristics stage (S7).

**De-risking (allowed).** The ABI is language-independent
(`c-extension-abi.md` §9), so the dynamic loader + a tier MAY be validated
against the existing **Zig** runtime first to separate *loader risk* from *port
risk*. The end state is the Rust runtime, AOT-first, passing all three tiers
and the Tcl 9 gold-standard suite.

---

## Component status table — `runtime/zig/` → Rust

Status vocabulary: **not-started** / **partial** / **landed**. "Gate" is the
concrete artifact that proves the component (a test, a sweep delta, a parity
entry). Anchor: every row is **not-started** at branch point (`runtime/rust/`
does not exist yet); the spike code is *not* a port and does not count toward
any row.

| Zig module | Files (lines) | Role | Rust target | Status | Gate that proves it |
|---|---|---|---|---|---|
| `valtypes/tcl_obj.zig` | 1 (1104) | `Tcl_Obj` model, refcount, shimmer | `runtime/rust/` obj core | not-started | leak_sweep zero-residual on round-trip + parity |
| `valtypes/` value types | 20 (9211) | list, dict, string, array, arith, format, encoding, hash_table, bs, chars, regex, arena, parse_cache | `runtime/rust/` valtypes | not-started | per-type unit tests mirror `runtime/zig/test_tcl_*.zig`; tcltest sweep no-regress |
| `parse/` | 3 (956) | `tcl_parse`, `tcl_subst` | `runtime/rust/` parse | not-started | parse/subst unit parity vs Zig + `tclParse.c` edge cases |
| `interp/tcl_interp.zig` | 1 (2065) | eval loop, interp object | `runtime/rust/` interp | not-started | eval-loop tcltest sweep no-regress |
| `interp/` frames/ns/procs | 8 (6348) | frames, namespaces, procs, catch, caps, trace, interp_registry | `runtime/rust/` interp | not-started | `test_tcl_frames/ns/procs` parity + namespace-tree doc |
| `dispatch/` | 5 (746) | cmd registry, cmd table, dispatch, diag, stub_fallback | `runtime/rust/` dispatch | not-started | `make check-wasm-parity` green |
| `cmds/` builtins | 34 (8367) | all builtin commands | `runtime/rust/` cmds | not-started | per-command parity + tcltest sweep per `.test` |
| `io/tcl_chan.zig` | 1 (1858) | channel subsystem | `runtime/rust/` io | not-started | chan/chanio/io/ioCmd tcltest suites (Memchan needs this) |
| `io/tcl_clock.zig` + `tcl_tz.zig` | 2 (3560) | clock + tz (+ `data/tzdata.bin`) | `runtime/rust/` io | not-started | clock tcltest slice (`run_clock_tcltest.py`) |
| `io/tcl_fs.zig` | 1 (1186) | filesystem (tclvfs needs `Tcl_FSRegister`) | `runtime/rust/` io | not-started | fs tcltest + tclvfs tier-1 gate |
| `sched/` | 7 (1660) | scheduler, coro, timer, vwait, fileevent, ready, asyncify | `runtime/rust/` sched | not-started | coroutine/after/vwait tcltest |
| `stubs/` | 6 (609) | env/fmt/fs/io/time stub surfaces | `runtime/rust/` stubs | not-started | covered by dependent command parity |
| `tcl_runtime.zig` (root) | 1 | export-aggregation root | `runtime/rust/` lib root | not-started | runtime builds + exports the `tcl_*`/`obj_*` symbol set codegen imports |
| `regex_include/` (C) | — | Henry Spencer ARE engine (C, vendored) | **kept as C** (`c-extension-abi.md` §10) | n/a | first C library compiled against the runtime; ARE fidelity |

`data/tzdata.bin` is a data asset consumed by the clock/tz port, not code.

> Update this table every PR: flip a row to **partial**/**landed** with its gate
> the moment it lands. Add new rows if a Zig refactor introduces a module.

---

## Track 1 — Rust runtime port

Goal: a `runtime/rust/` that the AOT codegen links against, parity-green, with
no leak/tcltest regression vs the Zig baseline.

- **T1.1 — Real `TclObj` + refcount discipline.** Mirror
  [`memory-management.md`](memory-management.md) +
  [`refcount-contract.md`](refcount-contract.md), cross-checked against
  `tclObj.c`. **Not** the leaking spike version. Gate: round-trip
  (`Tcl_NewObj` → incr → set-result → decr) shows zero residual under leak
  instrumentation.
- **T1.2 — parse/subst.** Port `parse/tcl_parse.zig` + `tcl_subst.zig` using
  `tclParse.c` for semantics. Gate: parse/subst unit parity.
- **T1.3 — eval loop + frames.** Port `interp/tcl_interp.zig` +
  `tcl_frames.zig`. Gate: eval-loop tcltest sweep no-regress.
- **T1.4 — namespaces + command table.** Port `tcl_ns.zig` + `dispatch/`.
  Gate: `make check-wasm-parity` green; namespace-tree behaviour preserved.
- **T1.5 — builtins.** Port `cmds/*.zig` incrementally, each command (or small
  group) one PR with its tcltest delta.
- **T1.6 — re-export the codegen ABI.** The AOT codegen imports a fixed set of
  `tcl_*`/`obj_*` primitives; the Rust runtime must export the same names/sigs
  so the parity check and the compiled-script harness stay green.

**Track 1 gates:** `make check-wasm-parity` green; the Tcl 9 suite
(`scripts/run_tcl9_tcltest_sweep.py`) + leak-check
(`scripts/leak_sweep.py` / `make leakcheck`) do **not** regress vs the Zig
baseline.

---

## Track 2 — Production C dynamic-linking interface

Promotes the spike into shipped infrastructure. Tracks the open items in
[`c-extension-abi.md`](c-extension-abi.md) §13 — flip them here as they land.

### T2.1 — C-API ownership / error contract (§13.1) — **land first**

A contract doc (sibling to `refcount-contract.md`) that, for every public
C-API function we ship, states its refcount category (callee-consumes /
callee-borrows / returns-owned-`+1` / returns-borrowed) and error-path
behaviour (`errorCode`/`errorInfo`/`Tcl_SetErrorCode`/return codes),
transcribed from `tmp/tcl9.0.3/doc/*.3` + the C source, mapped onto
`refcount-contract.md`. Plus a **gate that rejects a new C-API export lacking
an ownership annotation**.

- Status: **not-started.**
- Acceptance: every shipped C-API function carries an ownership category; the
  round-trip extension shows zero residual under the `-Dleak-check` counter.

### T2.2 — Shipped headers (§4.1, §7, §11)

Promote `runtime/rust-spike/include/{tcl.h,tclOO.h,tclTomMath.h}` to shipped
headers, widened to the full public-survey surface, backed by real impls. Ship
the full versioned `Tcl_ChannelType` / `Tcl_Filesystem` / `Tcl_ObjType` bodies
(the spike carries only probed fields). Status: **not-started.**

### T2.3 — Production dynamic loader (§5.2, §11)

Move the loader from the Python spike into the runtime/host. Parse `dylink.0`;
allocate `__memory_base`/`__table_base` from shared memory + the growable
table; resolve `GOT.mem.*` / `GOT.func.*` (the 4 `pkgooa` symbols characterise
the space — address-of-runtime-symbol); run `__wasm_apply_data_relocs` +
`__wasm_call_ctors` + `Foo_Init`. Status: **not-started.**

### T2.4 — Real-compiler dispatch (§13.2)

AOT-compiled user code resolves and calls an extension-registered command via
the runtime command table. Add the "register external command → shared-table
index" entry the dispatch needs. Status: **not-started.**

### T2.5 — Nominal stub tables (§6)

A real struct populated with our function pointers for the rare
stubs-introspection pattern (`pkgooa.c`). Status: **not-started.**

---

## Track 3 — AOT-first execution & whole-program link

Make the AOT compiler the primary path and link the whole program (runtime +
compiled user code + C extensions) into one artifact.

### T3.0 — Codegen command registry + backend-agnostic emit protocol — **foundational**

AOT codegen has to know **how to emit each Tcl command** into the target
instruction stream, and today that per-command knowledge is split across
backend-specific code (`core/compiler/codegen/bytecoded/` for the tclvm
bytecode VM, `core/compiler/codegen/wasm/` for WASM, each with its own
`_emitter` / `_imports` / `_statements`). The two emitters re-derive the same
command semantics independently, which is exactly the kind of drift the parity
gate exists to catch — but parity is a *cross-check*, not a *shared source*.

The port needs a **command-emission registry** distinct from the existing
command **spec** registry (`core/commands/registry/tcl/`, which is dialect/lint
metadata): a registry keyed by command (and sub-command) whose entries describe
how to lower that command, behind a **single backend-agnostic emit
protocol/trait** so one registration can target **any** backend:

- **tclvm** — the existing bytecode VM (`codegen/bytecoded/` → `opcodes.py`).
- **wasm** — the AOT WASM emitter (`codegen/wasm/`), the north-star path.
- **llvm ir** — a future native/JIT backend.

Shape (Rust trait, mirrored by the Python transitional surface):

```
trait CommandEmitter {                  // one impl per backend
    fn emit_call(&mut self, cmd: &ResolvedCommand, args: &[IrValue]) -> EmitResult;
    fn emit_builtin(&mut self, op: BuiltinOp, ...) -> EmitResult;   // set/incr/expr/list/...
    fn emit_dispatch_fallback(&mut self, name: &IrValue, argv: &[IrValue]) -> EmitResult;
}
// CommandEmitRegistry: command -> lowering rule, parameterised over the backend.
```

Each command registers its lowering **once**, against the trait; the WASM,
tclvm, and (future) LLVM backends are interchangeable implementations of the
trait. This is the codegen-side analogue of the runtime's "one command table":
the AOT compiler resolves a command to a lowering rule the same way the runtime
resolves it to a `CmdEntry`, and an extension-registered command (no static
lowering) falls through to `emit_dispatch_fallback` → the runtime command table
(§4.6 in `c-extension-abi.md`), which is also where the metaprogramming-S7
fallbacks land.

**Tie it to the editor command registry (single source of truth).** The
emit-lowering rule must be **bound to the same command registry the editor
uses** (`core/commands/registry/tcl/` — the spec/lint/hover/completion data),
so the set of commands the editor knows about and the set the compiler can emit
**cannot drift**. Preferred shape: the lowering rule *lives in* (or is
registered against) that registry as one more facet of a command's entry —
alongside its signature/dialect/lint metadata — rather than in a parallel table
that has to be kept in sync. This makes the existing `make check-wasm-parity`
cross-check a *consequence* of one source of truth rather than the thing holding
two tables together.

**Not every command has an emit impl yet — that's an explicit, well-formed
error.** A command can exist in the registry (so the editor lints/completes it)
without yet having a lowering rule for a given backend. Compiling a script that
*uses* such a command must raise a **clear compile-time error/exception**
(e.g. `NoEmitImpl{ command, backend }`) naming the command and backend — never
a silent miscompile, a panic, or a fallthrough that pretends success. This is
the codegen analogue of the runtime's trapping stub
(`dispatch/tcl_stub_fallback.zig`): a registry entry with no backing emitter is
a known-missing capability, surfaced loudly, and is distinct from an
*extension-/runtime-registered* command (which legitimately has no static
lowering and instead routes through `emit_dispatch_fallback` → the runtime
command table). The two must not be conflated: "no emitter for a builtin we
should support" is an error to fix; "no static lowering for a dynamically
registered command" is the designed dispatch path.

- Status: **not-started.** Today's per-backend emitters are the starting point;
  T3.0 factors their shared command knowledge behind the trait and binds it to
  the editor command registry.
- Why it belongs in this effort: AOT-first means the WASM emitter is the primary
  path, and linking C extensions adds a *third* class of command (runtime-/
  extension-registered) the emitter must dispatch uniformly. A backend-agnostic
  registry keeps tclvm (the oracle), wasm (the target), and a future llvm-ir
  backend emitting from **one** source of per-command lowering truth instead of
  N drifting copies guarded only by the parity cross-check — and binding it to
  the editor registry guarantees editor/compiler alignment by construction.
- Gate: WASM and tclvm backends emit from the shared registry with
  `make check-wasm-parity` green and no tcltest regression; the trait has ≥2
  live backend impls (wasm + tclvm) so the abstraction is proven, not
  speculative, before an llvm-ir impl is attempted; compiling a script that uses
  a command with no lowering rule for the active backend raises the
  `NoEmitImpl{ command, backend }` error (covered by a test), not a silent
  miscompile.

### Remaining Track-3 chunks

- **T3.1 — extension linking in `wasm_link.py`.** Extend
  `core/compiler/codegen/wasm_link.py` to also link extension objects — static
  Model A where possible, dynamic Model B otherwise.
- **T3.2 — drive AOT coverage up the staircase** so non-metaprogramming
  programs compile **100% AOT** (interpreter fallback never reached). Track in
  the [AOT-coverage scoreboard](#aot-coverage-scoreboard) below.
- **T3.3 — S7: metaprogramming heuristics (new staircase stage, beyond S6).**
  Heuristics that AOT-compile common metaprogramming patterns — `eval`/`subst`
  of statically-known scripts, list-built command/arg construction, constant
  `uplevel`/`upvar`/`namespace` forms — each **proven-safe or it falls through
  to the interpreter** (staircase rule: emit static WASM only where behaviour is
  provable, else fall back). Spec as a new `wasm-aot-staircase-s7.md` stage doc.

### AOT staircase context

S0–S6 are landed/partial on the compile side (see
[`wasm-aot-staircase.md`](../compiler/wasm-aot-staircase.md) stage skeleton):
S0–S2 landed, S3 partial, S4–S6 landed. **S7 (metaprogramming heuristics) is
the new stage this effort adds.** It obeys the same staircase rule — static
WASM only where provable, else interpreter fallback — so it never regresses
correctness, only widens the AOT surface.

### AOT-coverage scoreboard

Share of a representative corpus that **fully AOT-compiles** (zero interpreter
fallback at runtime). Seeded empty — baseline to be captured once T3.1 lands the
measurement harness.

| Corpus | Fully-AOT share | Falls back (why) | Notes |
|---|---|---|---|
| _seed — to be captured_ | — | — | establish baseline with T3.1 |

#### Metaprogramming-heuristic backlog (S7)

| Pattern | AOT heuristic | Fallback trigger | Status |
|---|---|---|---|
| `eval`/`subst` of statically-known script | compile the known script inline | non-constant script body | not-started |
| list-built command/arg construction | resolve the command + args at compile time | dynamic command name | not-started |
| constant `uplevel`/`upvar` forms | static frame resolution | dynamic level/var name | not-started |
| constant `namespace eval` body | compile body in target ns | dynamic ns name | not-started |

---

## Extension tier gates

Each tier is a PR series: vendor real extensions byte-identical with
provenance/licence, extend the compile-check, add LOAD+RUN tests under
`wasmtime`. Never merge a tier without its gate green.

### Tier 0 — in-tree dltest (9 samples)

All 9 `tmp/tcl9.0.3/unix/dltest/` samples LOAD and RUN: `pkga`, `pkgb`, `pkgc`,
`pkgd`, `pkge`, `pkgt`, `pkgua`, `pkgπ`, `pkgooa`. (`embtest.c` excluded — it
*embeds* Tcl, the opposite of extending it.)

| Sample | Exercises | LOAD | RUN |
|---|---|---|---|
| `pkga` | command/obj/result/UTF core | ☐ | ☐ |
| `pkgb` | int/wide accessors, `Tcl_AppendResult`, `Tcl_EvalEx` | ☐ | ☐ |
| `pkgc` / `pkgd` | int accessor + string/int obj results | ☐ | ☐ |
| `pkge` | error-returning init | ☐ | ☐ |
| `pkgt` | Tcl 9 `Tcl_*ObjCmd2` (`Tcl_Size` arity) | ☐ | ☐ |
| `pkgua` | load/unload + hash tables + thread-data | ☐ | ☐ |
| `pkgπ` | non-ASCII init naming | ☐ | ☐ |
| `pkgooa` | the GOT path + nominal stub table | ☐ | ☐ |

Status: **not-started** (spike compiles them; production LOAD+RUN not yet).

### Tier 1 — small real extensions (libc-only)

| Extension | Exercises | package-require + round-trip |
|---|---|---|
| Memchan | channel driver API (needs `io/tcl_chan` port) | ☐ |
| tclvfs | `Tcl_FSRegister` (needs `io/tcl_fs` port) | ☐ |
| tcllib critcl digest (sha1c/md5c) | custom `Tcl_ObjType` + byte arrays | ☐ |

Status: **not-started.** Large prerequisite surfaces (channels, VFS) land as
their own gated PRs first.

### Tier 2 — flagship sqlite3/tclsqlite

Acceptance: `package require sqlite3; sqlite3 db :memory:; db eval {create
table t(x); insert into t values(42); select x from t}` returns `42` under
`wasmtime`, against the **Rust** runtime via the loader — with the surrounding
script **AOT-compiled**. (amalgamation already builds to WASM; `tclsqlite.c` is
`tcl.h`-only.) Prerequisite: eval-loop depth for `db eval`, landed as its own
gated PR. Status: **not-started.**

---

## Tcl 9 test-suite scoreboard (gold standard)

`tmp/tcl9.0.3/tests/*.test` (168 files), run via
`scripts/run_tcl9_tcltest_sweep.py`. **In scope: behaviour.** No file passing on
the Zig baseline may regress. Per-file pass/partial/excluded is captured against
the Zig baseline; seeded empty here — the first sweep establishes the baseline
column.

| `.test` file | Zig baseline (pass/total) | Rust (pass/total) | Status |
|---|---|---|---|
| _seed — captured by first `run_tcl9_tcltest_sweep.py` run_ | — | — | not-started |

### Out-of-scope exclusions (by design)

These assert things our implementation **cannot match by design** — we emit
WASM, not Tcl bytecode, and own a different allocator/representation. Excluded
at **test granularity** (most of these files also contain in-scope behavioural
tests that stay in scope); only the specific constraint-bearing tests are
excluded.

| Exclusion class | Mechanism | Affected files (Tcl 9.0.3) | Rationale |
|---|---|---|---|
| Representation / shimmering | `tcl::unsupported::representation` | `abstractlist`, `expr`, `format`, `history`, `lrange`, `lseq`, `string`, `uplevel` | internal repr is an impl detail; we shimmer differently |
| Bytecode / disassembly | `tcl::unsupported::disassemble` / `getbytecode` | `compExpr`, `compile`, `namespace` | we emit WASM, not Tcl bytecode |
| Memory introspection / allocator | `memory` command | `apply`, `assemble`, `basic`, `cmdIL`, `compExpr`, `compile`, `coroutine`, `dict`, `env`, `error`, `fileName`, `for`, `listObj`, `namespace`, `oo`, `ooNext2`, `parse`, `proc`, `regexp`, `string`, `trace`, `var` | internal allocator layout / `memory` introspection is not matchable |

> When a file is fully excluded (vs per-test), record it here with the reason.
> The sweep harness (`run_tcl9_tcltest_sweep.py`) and the excluded set are the
> authority; this table mirrors it.

---

## Upstream sync log (Zig → Rust)

The Zig runtime keeps getting fixed during the port. On a cadence, diff
`runtime/zig/` against the last-synced commit and record dated sync /
gap-audit entries (mirroring `rust-rewrite.md`'s SYNC-* / GAP-AUDIT-*
discipline), noting which behavioural changes have been mirrored into Rust.

**Audit workflow** (run before each chunk and on each SYNC family):

```
git fetch origin
git log --oneline <last-synced>..origin/rust -- runtime/zig/   # Zig changes since last sync
git diff --stat <last-synced>..origin/rust -- runtime/zig/      # impact
```

Classify each Zig commit: **out-of-scope** (Zig-only infra, build) → record and
skip; **in-scope behavioural** (a fix in a module already ported to Rust) → add
an Outstanding row with the source commit + the Rust file(s) to update; mirror
it in the same or a follow-up PR.

### SYNC anchor — 2026-06-05 (branch point)

- Last-synced commit: `rust`@`8150eca` (#549, the spike merge).
- `runtime/rust/` does not exist yet, so **nothing to mirror** — every
  component is not-started and tracks the Zig source as-of this anchor.
- Action: the first Track-1 PR that creates `runtime/rust/` records its Zig
  source baseline (per-module commit) so subsequent diffs are precise.

### Outstanding

_(empty — populated as Zig lands behavioural fixes during the port)_

| Date | Zig commit | Module | Behavioural change | Mirrored into Rust |
|---|---|---|---|---|
| — | — | — | — | — |

---

## Gates summary

| Gate | Command | Applies to |
|---|---|---|
| WASM command parity | `make check-wasm-parity` | Track 1 (registry/dispatch/builtins) |
| Tcl 9 tcltest sweep | `scripts/run_tcl9_tcltest_sweep.py` | Track 1, Tier gates, correctness gold standard |
| Leak sweep | `scripts/leak_sweep.py` / `make leakcheck` | Track 1 (refcount discipline), T2.1 |
| Tier LOAD+RUN | per-tier `wasmtime` tests | Tier 0/1/2 |
| C-API annotation | T2.1 export-annotation gate | Track 2 |
| AOT coverage | T3.1 coverage harness | Track 3 |

No `.test` file that passes on the Zig baseline may regress. `make
check-wasm-parity` and the editor extensions stay green — do **not** regress the
compiler/LSP or the Zig runtime.

---

## Next-up priority queue

1. **This document** (the first deliverable) — establish + keep current.
2. **T2.1** — C-API ownership/error contract + un-annotated-export gate
   (largest remaining *design* gap; unblocks leak-correct extensions).
3. **T1.1** — real `TclObj` + refcount core in `runtime/rust/` (records its Zig
   source baseline for the sync log).
4. **T3.0** — backend-agnostic emit protocol/trait + command-emission registry
   bound to the editor command registry; `NoEmitImpl` error for unimplemented
   commands (the codegen-side single-source-of-truth that all later AOT work
   builds on).
5. **T2.3** (de-risk against Zig first) — production loader, validated on
   Tier 0 dltest, separating loader risk from port risk.
6. **T3.1** — `wasm_link.py` extension linking + AOT-coverage measurement
   harness (seeds the scoreboard).
7. **S7 spec** — `wasm-aot-staircase-s7.md` (metaprogramming heuristics).

---

## Conventions

- Keep **this doc and `c-extension-abi.md` current every PR** (flip §13 items
  as they land; log every upstream Zig sync).
- Add KCS / design docs per [`AGENTS.md`](../../../AGENTS.md); commits scoped
  and gated.
- Never merge a tier or stage without its gate green.
- If a needed surface is large, land it as its own gated PR before the gate that
  needs it.

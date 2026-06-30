# Python → Rust port gaps — audit

> **Audience:** Maintainer / Contributor
> **Type:** Design (status audit)

A single consolidated inventory of **every feature not yet completely ported
from Python to Rust**, verified against source on the day of writing:

- Rust side: `rust` branch, HEAD `0d25edb1` (2026-06-27).
- Python oracle: `main`, HEAD `717815ad` (2026-06-25).

> **Note on links:** this report lives on a `main`-based branch (the Python
> tree). The Rust workspace and its design docs live on the **`rust` branch**.
> Referenced docs are given as `rust`-branch paths in plain code spans rather
> than as relative links, because they do not exist in this branch's tree.

This audit cross-checks the living plan in `docs/rust-rewrite.md` (on the `rust`
branch) against the actual crate and package source. It found the plan
**accurate and current** — no subsystem is
missing a tracker row, every top-level Python package (`shared/`, `compiler/`,
`dialects/`, `analyser/`, `server/`, `tooling/`, `ai/`) maps to a crate or an
explicit "stays Python" decision, and all dialect spec packs (`tcl`, `tcllib`,
`stdlib`, `expect`, `tk`, the five `eda/<vendor>` packs, and `f5/*`) are present
under `rust/tcl-registry/src/commands/`. What follows is the gap list, ordered
by the plan's five dependency stages, with the specific residual for each.

Status legend (from the plan): ✅ done · 🟢 done bar listed residuals ·
🟡 partial · 🔴 not started.

## Executive summary

The foundation (lexer, segmenter, CST, IR/CFG/SSA, dataflow, type/shimmer,
var-escape, optimiser + inliner, analyser diagnostics including all F5 families,
regex engine, the LSP server/core/db, and most tooling) is **ported and
verified against the Python oracle**. The remaining gaps cluster into a small
number of areas:

| # | Gap | Track | Status | Size |
|---|---|---|---|---|
| 1 | WASM codegen emitter + `tcl-wasm` bundling | RT-WASM | 🟡 | **Largest single gap** |
| 2 | Bytecode VM command surface + tcltest parity | RT-VM | 🟡 | Large |
| 3 | `runtime/rust` tree-walking runtime port | (runtime, off-workspace) | 🟡 | Large |
| 4 | Per-edit / cross-file incrementality (Tasks 1/2/6 shipped; 3/4/5/7 remain) | SRV-INCREMENTAL | 🟡 | XL |
| 5 | PyO3 public API finish + Python retirement | API-PYO3 | 🟡 | Large |
| 6 | Bytecode codegen bare-statement specialisations | FE-CODEGEN | 🟢 | Small |
| 7 | Tooling residuals (explorer, fuzzer, irule-test, regex cmd-plumbing) | TOOL-* | 🟢 | Small |
| 8 | FP precision divergences (5 `#[ignore]`s) + astral-output columns | FE-DIAG/FP | residuals | Small |

Everything else in the plan's subsystem table is ✅.

---

## Stage 1 — Front-end residuals

### 6. FE-CODEGEN — bytecode codegen 🟢

`tcl-compiler::codegen` (non-wasm). State-mutating statement-position
specialisations, `expr` const-folding, byte-wise disassembly escaping, the
`set x [cmd]` inline assign, non-proc `dict` mutators (ensemble `invokeReplace`),
and `{*}` expansion inside command substitutions have all landed byte-true vs
tclsh 9.0. **Residual:**

- Statement-position (value-discarded) specialisations for the *value-returning*
  commands used as bare statements — `string` / `regexp` / `lindex` /
  `lreplace`. The value-position inline forms already exist, so this is
  value-emit + `pop`, gated on threading the per-arg braced-flag through the
  hook. Low frequency; `regexp` with match-vars is the one with real
  statement-position semantics.

### FE-OPT — optimiser 🟢 (out-of-scope residual)

Every O-code pass and the full inliner (v0 verbatim **and** v3 α-rename +
parameter binding + return-as-break wrap) are ported. Two items are explicitly
**not** counted as port gaps:

- v3's execution-differential verification is owned by its consumer (RT-WASM).
- The optional, non-correctness **O110** rewrites are gated on the iRules
  `MatchesGlob` / `MatchesRegex` expr operators landing first.

### FE-DIAG / FP precision — analyser false-positive divergences (residuals)

The analyser diagnostics families are ✅ and consumer-wired. The FP precision
catalogue port (`rust/tcl-compiler/src/analyser/diagnostics/fp/`, 343 tests
passing) carries a small set of **genuine Rust-vs-Python divergences** still
held as `#[ignore]` (see `docs/design/rust/fp-rust-port-status.md` on the `rust`
branch for the live worklist):

- **FP-OBJ-10** — callback-array-slot `W307` suppression vs its SCCP-const TP
  variants (IR-shape of a direct `set arr(key) literal`).
- **FP-OBJ-D4-F5** — const value not captured in an `oo::class` method-body scope
  (`W307` should fire).
- **FP-OPT-03** — LICM does not hoist an outer-pure/inner-pure nested
  `[format … [expr {…}]]` (`O106` not emitted).
- **FP-OPT-08** — overlap arbitration deletes `set b 0` while a `$b` survives in
  the EXPR role.
- **astral-output C1** — provider *output* columns
  (`document_highlight`, go-to-definition target ranges) count Unicode scalars
  rather than UTF-16 code units, so an astral char (🚀) yields a column one short.
  The analyser stores `var_def.references` / proc `name_span` byte-spans that are
  themselves miscounted; the LSP range-lift is already correct.
- **FP-NAB-03 / FP-NAB-12** — internal-API coverage (interproc `pure` summary;
  `is_pure_var_ref` value-shape parser) needs equivalent Rust unit tests, not a
  diagnostic.

---

## Stage 2 — Runtime & execution

### 1. RT-WASM — WASM codegen + runtime 🟡 — **largest single gap**

Owns `tcl-compiler::codegen::wasm`, `runtime/zig`, and a new `tcl-wasm` bin.
The eval-fallback emitter and `tcl compwasm` wiring have landed (binary/WAT
output, `wasmtime`-validated). **Scale of the gap:** the Rust emitter is
~1.5 K LOC across 4 files
(`codegen/wasm/{backend,encoding,ir,mod}.rs`); the Python package it must reach
parity with is **~20.6 K LOC across 49 modules** (`compiler/codegen/wasm/`),
including a per-command emitter for each of
`append`, `apply`, `array`, `catch`, `clock`, `concat`, `dict`, `error`,
`fconfigure`, `fcopy`, `format`, `info`, `lappend`, `linsert`, `list`,
`lreplace`, `lsearch`, `lsort`, `puts`, `regexp`, `return`, `runtime`, `scope`,
`set`, `string`, and `uplevel`. **Remaining:**

- **(large)** Finish the WASM emitter (`wasm_codegen_module`) — only the Phase-1
  IR + encoding is ported vs the ~13-module Python emitter package.
- The `IRInterpBoundary` IR node + its insert pass.
- The IR-rewriting `passes/dce.py` and `passes/gvn.py`.
- `source_inliner` / `stdlib_prelude` (WASM-bundle self-containment).
- The `tcl-wasm` CLI + `--link` (Binaryen) bundling — standalone self-contained
  module.

### 2. RT-VM — bytecode VM 🟡

Owns `tcl-vm` (+ the `tcl-vm-cli` / `tclvm` driver). The engine core is solid
(loads the real Tcl 9 `tcltest.tcl` end-to-end) and the **VM-vs-tclsh
differential `bug_*` cmd-tests are all closed** (2026-06-25: math/`expr`,
`string`/`format`, dict/array/`lsort`, `try`/control, `namespace which
-variable`). **Open — tcltest-suite parity vs the more-complete `runtime/rust`,
both pinned against C Tcl 9.0.3:**

- **(P1) Uncaught-error abort / error-propagation gap.** A test-body error that
  escapes tcltest's `catch` propagates to the module top and aborts the whole
  `run_test` driver (e.g. `info.test` halts at info-8.3, `proc.test` on a bare
  `VM error:`). This — an `uplevel`/`catch`/error-propagation gap, not a
  deadlock — is what zeroes the remainder of those suites. (Re-baseline the
  parity harness in `--release` first: debug-build slowness × the harness
  timeout masquerades as a hang.)
- `namespace` / `var` / `upvar` **depth** (~290 failures combined) —
  namespace-name canonicalisation of multiple/trailing `::` runs; deeper
  variable-scoping / introspection. The three share the model.
- `error.test` (29 failing) — 22 are `[try]`-coverage tests needing the
  **`-level` countdown** (only `-level 0` is immediate today); the rest are
  `info errorstack` / the `-errorstack` option (unimplemented) and errorInfo
  edge cases.
- Smaller per-suite gaps — `switch` (14), `for` (8), `foreach` (6), `incr` (3),
  `if`/`set` (2), `while` (1).
- **Structural** — give the bytecode backend real exception ranges
  (`beginCatch`) and a fixed nested-complex-`foreach` / `lmap`-collecting
  codegen, then drop the `for_bytecode` barriers (`try` / nested-`foreach` /
  `lmap` run via runtime builtins today — correct but not inline).
- **Missing command surface** — **TclOO** (largest), `clock`, full `interp`,
  real I/O (`open` / `gets` / `seek`), `after` / `vwait`, **coroutine**, and
  residual `file` / `info` / `namespace` subcommands. Concretely, `info` is
  missing `cmdcount` / `frame` / `functions` / `hostname`. (`info cmdcount`
  cannot reach exact-count parity without per-bytecode command counting — an
  "exists but approximate" subcommand.)

### 3. `runtime/rust` — tree-walking runtime port 🟡 (off-workspace)

`runtime/rust/` is the standalone Rust port of the Zig WASM runtime (the
`runtime/zig/` tree), kept out of `cargo test --workspace` (it needs raw-pointer
`unsafe` over shared linear memory) and gated via `make runtime-rust-test`. It
is the eventual in-process tree-walking interpreter and the wasm32 runtime the
emitted modules link against. Per
`docs/design/runtime/rust-runtime-port.md` (on the `rust` branch), most
subsystems are **partial**:

- **`TclObj` + refcount / shimmer** (T1.1) — partial; round-trips leak-clean,
  but the full typed-rep machinery is incomplete.
- **valtypes** — list, dict, and string capacity/char-ops landed; **array,
  arith, format, encoding, hash_table, bs, chars, arena, parse_cache** still to
  port (each with a representation-decision note).
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
  library to `package require tcltest`), but the tcltest sweep is incomplete:
  e.g. `dict.test` 272/373, `lrange` 1759/1766; per-command parity ongoing.
- **regex** — the **C** Henry-Spencer ARE engine is linked today (`have_regex`,
  `build.rs` + FFI shim); the **Rust port of the algorithm is the end-stage
  swap** (`tcl-regex` already exists workspace-side, so this is a convergence,
  not a fresh port).
- **coroutines** — native stack-swap + threads are `cfg`-gated off under the
  `BrowserHost` capability host on wasm32.
- **lib root / ABI** — compiles and links for `wasm32-unknown-unknown` and an
  emitted module runs against it end-to-end, but the **rest of the
  `tcl_*`/`obj_*` symbol set** and **true PIC** (drop the per-program reserved
  constant-pool gap for `__memory_base`) remain.

---

## Stage 3 — Server

### 4. SRV-INCREMENTAL — per-edit / cross-file incrementality 🟡

The LSP server/core/db (SRV-LSP) is ✅. The per-edit incremental pipeline is now
**substantially shipped** — the original 🔴 was stale. Measured headline: warm
per-edit latency on `linalg.tcl` started at ~411 ms, of which whole-file
`run_all_checks` was ~405 ms (~99%). The track's seven tasks:

1. ✅ Persisted incremental `LineIndex` on the `String` store (no rope).
2. ✅ **The prize** — per-function check memo (`function_checks`, 2a) **and**
   incremental interprocedural-taint memo (`proc_taint_solve` /
   `proc_summary_cascade`, 2b). Warm `compiler_check_diagnostics` fell ~445 → ~83 ms.
3. 🔴 Approach A — incremental per-item IR lowering (the ~59 ms lowering floor;
   blocked by whole-module passes coupling one body's IR to others).
4. 🟡 Approach B follow-ups — deep-clone removal *skipped* (measured 0.1 ms, below
   the value bar); per-function `optimise_unit` memo not yet built (M).
5. 🔴 Wire `reparse_window` into the live re-lex path — *blocked*: no windowed
   re-lex consumer exists; coupled to Task 7's chunk-addressable input.
6. ✅ **Cross-file cascade** — `Project` salsa input, `project_proc_names` /
   `project_command_arities`, `project_diagnostics` (W123 suppression + cross-file
   E002/E003 arity across procs/classes/aliases/ensembles), live server wiring,
   **plus the per-symbol `command_arity` early-cutoff** (a file's cross-file
   diagnostics depend only on the symbols it references, so an unrelated proc's
   signature edit no longer wakes it) **and the corpus-scale multi-file
   `incremental == fresh` fuzzer** (`project_diagnostics_corpus.rs`).
7. 🔵 **Optional, gated** — rope store + chunk-addressable `SourceFile` input,
   landing only if its 0.02% slice grows measurable *and* many-small-doc memory
   stays under ~1.2× (may legitimately never land).

**Genuinely remaining:** Task 3 (IR-lowering incrementality, L), Task 4's
`optimise_unit` memo (M), Task 5 (blocked on 7), Task 7 (optional/gated).

---

## Stage 4 — Tooling residuals

Most of Stage 4 is ✅ (TOOL-TCLPKG, TOOL-REFACTOR, TOOL-F5, TOOL-DEBUGGER,
TOOL-CLI, formatter/minifier/diagram). The 🟢 residuals:

### 7a. TOOL-EXPLORER 🟢 (gated on RT-WASM)

The compiler explorer's rich per-instruction web-GUI shape (`to_explorer_json` →
`tcl_explorer::wasm_explorer`) has landed (resolved call/branch targets,
block-pairing, ranges). It **densifies automatically as RT-WASM emits real
instructions** — there is no separate explorer work, only the RT-WASM dependency.

### 7b. TOOL-FUZZ 🟢 (gated on RT-WASM)

The differential fuzzer (campaign runner, seeded generator, findings registry,
`wasm-check` runnability arm, `wasm-diff` value-differential arm) has landed.
**Residual:** re-back the `wasm-diff` arm with the **real linked Zig runtime**
for a full value differential (gated on RT-WASM).

### 7c. TOOL-IRULE-TEST 🟢

SCF→orchestrator topology generator + `LiveSession` on `tcl-vm` landed (14
integration tests green). **Residual:** auto-broadening coverage only.

### 7d. Regex command-plumbing 🟢

The ARE engine (`tcl-regex`) is ✅ (passes `reg.test` 544/544). **Residual,**
living in `tcl-cmd-core`: the `-about` option, `regsub -command`, and
`-start`-validation cmd-plumbing gaps.

---

## Stage 5 — Public API & Python retirement

### 5. API-PYO3 🟡

The designed public facade surface has **landed**
(`parse_tcl` / `compile_tcl` / `analyse_tcl` / `format_tcl` /
`parse_bigip_config` / `query_bigip` + the `TclLspError` hierarchy, in
`tcl-lsp-py::public`). **Residual — three workstreams:**

- **TEST-MIGRATE** — port the pytest suite to cargo unit + integration tests
  (the 473-file classification has started); the legacy `tests/` directory
  shrinks to zero at the final retirement task.
- **`scripts` → `xtask`** — rewrite the build/release scripts under `scripts/`
  as `cargo xtask` subcommands (one verb already ported:
  `audit_option_dialects`).
- **PYTHON-RETIRE** — delete the in-tree Python once every consumer above has
  moved to Rust. This is intentionally **last** and can only close after Stages
  1–4 are complete.

### `ai/` (MCP server + Claude skills) — n/a

`ai/` **stays Python by design** — it is not a port gap.

---

## Notes on method and confidence

- The plan document (`docs/rust-rewrite.md`) was last refreshed at commit
  `7ec07ff9` (2026-06-25); the 442 commits between it and `rust` HEAD are
  predominantly CI/release plumbing plus minor fixes (BIG-IP code-action wiring,
  positional `%n$` format-spec handling, `try`-handler arity), none of which
  open or close a subsystem-level gap. The plan is therefore current.
- Where the audit could measure a gap directly it did: the RT-WASM emitter LOC
  ratio (1.5 K Rust vs 20.6 K Python / 49 modules) and the dialect-pack
  coverage (all `eda/<vendor>`, `expect`, `tcllib`, `tk`, `stdlib`, `tcl`, and
  `f5/*` packs present under `tcl-registry/src/commands/`) were verified from
  source rather than taken from the plan.
- Per-item evidence behind the gaps lives on the `rust` branch:
  `docs/design/rust/compiler-pipeline-parity.md` (FE residuals);
  `docs/design/rust/fp-rust-port-status.md` (FP worklist);
  `docs/design/runtime/rust-runtime-port.md` (runtime detail);
  `docs/design/srv-incremental/README.md` (incrementality design).
</content>
</invoke>

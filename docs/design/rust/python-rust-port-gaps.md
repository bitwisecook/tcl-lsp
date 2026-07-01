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

> **Scope:** this audit covers the **front-end / analyser / server / API /
> tooling** port gaps — the target is to finish these first. The **runtime &
> execution** layers (WASM codegen, the bytecode VM, and the `runtime/rust`
> port) are enumerated separately and completely in
> [`../runtime/runtime-execution-gaps.md`](../runtime/runtime-execution-gaps.md).

## Executive summary

The foundation (lexer, segmenter, CST, IR/CFG/SSA, dataflow, type/shimmer,
var-escape, optimiser + inliner, analyser diagnostics including all F5 families,
regex engine, the LSP server/core/db, and most tooling) is **ported and
verified against the Python oracle**. The remaining gaps cluster into a small
number of areas:

| # | Gap | Track | Status | Size |
|---|---|---|---|---|
| 4 | Per-edit / cross-file incrementality (Tasks 1/2/4/6 shipped; only Task 3 remains; 5/7 dropped — rope) | SRV-INCREMENTAL | 🟡 | M |
| 5 | PyO3 public API finish + Python retirement | API-PYO3 | 🟡 | Large |
| 6 | Bytecode codegen bare-statement specialisations | FE-CODEGEN | 🟢 | Small |
| 7 | Tooling residuals (irule-test, regex cmd-plumbing) | TOOL-* | 🟢 | Small |
| 8 | FP precision divergences (5 `#[ignore]`s) + astral-output columns | FE-DIAG/FP | residuals | Small |

The runtime & execution gaps (WASM / bytecode VM / `runtime/rust`) are tracked
separately (see the scope note above). Everything else in the plan's subsystem
table is ✅.

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

- v3's execution-differential verification is owned by its consumer (the
  execution/WASM track, tracked separately).
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

Moved out of this audit — the WASM codegen, bytecode VM, and `runtime/rust` gaps
are enumerated in
[`../runtime/runtime-execution-gaps.md`](../runtime/runtime-execution-gaps.md).

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
4. ✅ **Per-procedure `optimise_unit` memo** — optimisations assembled from a
   per-proc memo (`function_optimisations`) on single-procedure offset-0 CUs +
   whole-module `finalise_optimisations`, byte-identical to `optimise_unit`
   (893-file cold corpus + random-edit corpus + 250-edit in-crate fuzzers); an
   unrelated body edit re-optimises exactly one proc. (Approach B's deep-clone
   removal stays *skipped* — measured 0.1 ms, below the value bar.)
5. ⊘ **DROPPED** (rope-dependent, 2026-06-30 decision) — windowed re-lex is
   coupled to Task 7's chunk-addressable input; out of scope.
6. ✅ **Cross-file cascade** — `Project` salsa input, `project_proc_names` /
   `project_command_arities`, `project_diagnostics` (W123 suppression + cross-file
   E002/E003 arity across procs/classes/aliases/ensembles), live server wiring,
   **plus the per-symbol `command_arity` early-cutoff** (a file's cross-file
   diagnostics depend only on the symbols it references, so an unrelated proc's
   signature edit no longer wakes it) **and the corpus-scale multi-file
   `incremental == fresh` fuzzer** (`project_diagnostics_corpus.rs`).
7. ⊘ **DROPPED** (rope-dependent, 2026-06-30 decision) — rope store +
   chunk-addressable `SourceFile` input; the `String` store is retained. Out of
   scope.

Also shipped this session: the **Task 2b random-edit differential fuzzer** (the
named "still to build" verification gate — in-crate 250-edit + corpus `--ignored`,
asserting the memoised checks path stays byte-identical to a fresh build across
fuzzed edit sequences) **and Task 4 (the per-procedure `optimise_unit` memo)**.

**De-roped remaining (Tasks 5 + 7 dropped, Task 4 shipped 2026-06-30):** only
**Task 3** — incremental per-item IR lowering (the ~59 ms `CompilationUnit`
build floor; blocked by whole-module lowering passes that couple one body's IR to
others, resolved with the same "cross-item facts as inputs" split the analyser
walk used). Everything else in the de-roped track (Tasks 1/2/4/6) is shipped.

---

## Stage 4 — Tooling residuals

Most of Stage 4 is ✅ (TOOL-TCLPKG, TOOL-REFACTOR, TOOL-F5, TOOL-DEBUGGER,
TOOL-CLI, formatter/minifier/diagram). The compiler-explorer (TOOL-EXPLORER) and
differential-fuzzer (TOOL-FUZZ) residuals are **WASM-gated and tracked separately**
with the runtime track. The remaining 🟢 residuals:

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
- Where the audit could measure a gap directly it did: the dialect-pack
  coverage (all `eda/<vendor>`, `expect`, `tcllib`, `tk`, `stdlib`, `tcl`, and
  `f5/*` packs present under `tcl-registry/src/commands/`) was verified from
  source rather than taken from the plan.
- Per-item evidence behind the gaps lives on the `rust` branch:
  `docs/design/rust/compiler-pipeline-parity.md` (FE residuals);
  `docs/design/rust/fp-rust-port-status.md` (FP worklist);
  `docs/design/srv-incremental/README.md` (incrementality design). The
  runtime & execution detail (WASM / bytecode VM / `runtime/rust`) is tracked
  separately under `docs/design/runtime/`.
</content>
</invoke>

# Python → Rust port gaps — audit

> **Audience:** Maintainer / Contributor
> **Type:** Design (status audit — **historical**)

> **Historical: do not plan from this file.** It was written on 2026-06-27
> against a `main`-based branch while the Python tree still existed, and the
> world it describes is gone. Spot-checked on 2026-08-07, its executive summary
> is wrong in at least three places: Python is retired (there is no `pyproject.toml`
> or `uv.lock` on this branch, so the "PyO3 public API finish + Python retirement"
> row is closed — the only remaining `pyo3` dependency is the deliberate
> `rust/bigip-report-gen/python` binding); the "FP precision divergences
> (5 `#[ignore]`s)" row is closed, as the FP suite carries no `#[ignore]` at all;
> and the `f5-cli` tooling residual is down to a single verb (issue #1315).
>
> Kept for the record of *how* the port was sequenced. For live gaps use
> [`../runtime/runtime-execution-gaps.md`](../runtime/runtime-execution-gaps.md)
> (runtime and execution) and the GitHub issue list (everything else).

A single consolidated inventory of **every feature not yet completely ported
from Python to Rust**, verified against source on the day of writing:

- Rust side: `rust` branch, HEAD `0d25edb1` (2026-06-27); SRV-INCREMENTAL row
  refreshed 2026-07-01 (Task 3 gated v1 shipped).
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
| 4 | Per-edit / cross-file incrementality (Tasks 1/2/3/4/6 shipped byte-identical; 5/7 dropped — rope); residual: broaden the Task 3 body-cache gate | SRV-INCREMENTAL | 🟢 | S |
| 5 | PyO3 public API finish + Python retirement (incl. `scripts`→`xtask`: only 6 of ~26+ done; **`ai/` re-pointing off the retiring engine**) | API-PYO3 | 🟡 | Large |
| 6 | Bytecode codegen bare-statement specialisations | FE-CODEGEN | 🟢 | Small |
| 7 | Tooling residuals (irule-test event dispatch, formatter docstring rewriter, `f5-cli` irule sub-verbs + SSH + registry-dump `commands`, regex cmd-plumbing) | TOOL-* | 🟢 | Small–Med |
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

### 4. SRV-INCREMENTAL — per-edit / cross-file incrementality 🟢

The LSP server/core/db (SRV-LSP) is ✅. The per-edit incremental pipeline is now
**shipped** — the original 🔴 was stale. Measured headline: warm per-edit latency
on `linalg.tcl` started at ~411 ms, of which whole-file `run_all_checks` was
~405 ms (~99%). The track's seven tasks:

1. ✅ Persisted incremental `LineIndex` on the `String` store (no rope).
2. ✅ **The prize** — per-function check memo (`function_checks`, 2a) **and**
   incremental interprocedural-taint memo (`proc_taint_solve` /
   `proc_summary_cascade`, 2b). Warm `compiler_check_diagnostics` fell ~445 → ~83 ms.
3. ✅ **Approach A — incremental per-item IR lowering** (`lower_proc_body` memo
   keyed on `ProcBodyKey`, gated by `file_body_cache_eligible`). Byte-identical
   incremental == fresh over the corpus + in-crate fuzzers; an unrelated body
   edit reuses every other proc's lowered IR. The eligibility gate is
   conservative (disqualifies bodies touching
   `namespace`/`interp`/`rename`/OO/`apply`/nested-proc); broadening it to
   recover more reuse is the one remaining residual.
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
fuzzed edit sequences), **Task 4 (the per-procedure `optimise_unit` memo)**, and
**Task 3 (per-item IR lowering) gated v1**.

**Remaining (de-roped):** only a **residual** — broaden the Task 3 body-cache
eligibility gate (`file_body_cache_eligible`) to recover warm-edit IR reuse for
bodies it currently disqualifies, keeping the per-item memo byte-identical to a
whole-module lowering. Tasks 5 + 7 are dropped (rope-dependent); everything else
(Tasks 1/2/3/4/6) is shipped byte-identical.

---

## Stage 4 — Tooling residuals

Most of Stage 4 is ✅ (TOOL-TCLPKG, TOOL-REFACTOR, TOOL-DEBUGGER, TOOL-CLI,
minifier/diagram). The compiler-explorer (TOOL-EXPLORER) and differential-fuzzer
(TOOL-FUZZ) residuals are **WASM-gated and tracked separately** with the runtime
track. The remaining 🟢 residuals, verified against crate source:

### 7c. TOOL-IRULE-TEST 🟢

SCF→orchestrator topology generator + `LiveSession` on `tcl-vm` landed (14
integration tests green). **Residual:** auto-broadening coverage, **plus** the
session's `event dispatch` / `class match` handlers, which are not yet
implemented (`tcl-irule-test/src/session.rs`).

### 7d. Regex command-plumbing 🟢

The ARE engine (`tcl-regex`) is ✅ (passes `reg.test` 544/544). **Residual,**
living in `tcl-cmd-core`: the `-about` option, `regsub -command`, and
`-start`-validation cmd-plumbing gaps.

### 7e. Formatter — docstring rewriter 🟢

The formatter engine + minifier are ported (`tcl-lsp-core::{formatting,minify}`,
byte-parity). **Residual:** the **docstring rewriter is not yet implemented** —
its config flags are carried through `formatting::config` but the engine does
not consume them (`rust/tcl-lsp-core/src/formatting/config.rs`).

### 7f. TOOL-F5 — `f5-cli` residuals 🟢

The core `f5` verbs are ported (`event-order` / `extract` / `format` / `minify` /
`event-info`, plus `explain-flow` and `--simulate` on `tcl-vm`). **Residuals in
`f5-cli`:** the `irule lint` / `context` / `trace` / `pgo` sub-subcommands are
**not yet implemented** (they parse args so `--help` works, then error + exit 2 —
`f5-cli/src/commands/irule.rs`); the **SSH/scp fetch transport** is not ported
(REST works; an SSH request falls back to the Python CLI —
`f5-cli/src/commands/remote/ssh.rs`); and `registry-dump --section commands` is
not implemented (`f5-cli/src/commands/registry_dump.rs`).

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
- **`scripts` → `xtask`** — **essentially done.** The check/build verbs
  (`refcount-contract`, `kcs-index-links`, `version`, `tzdata-bundle`,
  `audit-option-dialects`, `diag-tables`) plus the full **editor/AI codegen**
  suite are now `cargo xtask` verbs generating from the Rust registries:
  `gen-editor-catalogs`, `gen-editor-settings` (`diagnosticCatalog.ts`),
  `gen-vscode-package` (`package.json`), `gen-jetbrains-catalog` (all 3 Kotlin
  files), `gen-ai-diagnostics` (`ai/shared/diagnostics.json` + the 4 AI
  prompt/skill files); `gen-kcs-db` is handled natively by `tcl-cli/build.rs`.
  **`scripts/codegen/editor_settings.py` is fully retired** (`render_all` empty).
  The remaining `scripts/*` are **Bucket A "retire-with-Python"** (measurement,
  Python↔Rust differentials, Python-oracle→Rust generators frozen at retirement)
  or **Bucket B4–B7 keep-forever** shell helpers — see
  [`scripts-retirement-triage.md`](scripts-retirement-triage.md). `gen-query-builtins-doc`
  is resolved as freeze-and-drop: the 184 KB of builtin *prose* is documentation
  that stays in `docs/` (the Rust `BuiltinSpec` omits prose by design), so its
  Python generator retires without a byte-exact Rust replacement.
- **PYTHON-RETIRE** — delete the in-tree Python once every consumer above has
  moved to Rust. This is intentionally **last** and can only close after Stages
  1–4 are complete. **Note the `ai/` coupling below** — it is a real consumer of
  the retiring engine, not an independent Python island.

### `ai/` (MCP server + Claude skills) — glue stays Python, but it is a consumer

The ai **shell** stays Python by design — the MCP JSON-RPC framing, the CLI
arg-parsing, the ~25 skill manifests (markdown), the prompt composition, and the
Jinja templates are language-agnostic glue that will not port.

**But `ai/` is not independent of the Python analysis engine that PYTHON-RETIRE
deletes.** 7 of 13 `ai/*.py` files import the in-tree engine directly —
`analyser` (23×), `tooling.refactoring` (20×), `compiler` / `compiler.registry`
(28×), `server.features` (8×), and `dialects.f5/xc/tk` (16×) — and they reach
**deep internals** (`semantic_graph`, `memory_ssa`, `taint`, optimiser internals,
`server.features.*`), well beyond the six landed `tcl-lsp-py::public` facades.
So retiring the in-tree Python forces one of: (a) expand the PyO3 surface to cover
`ai/`'s full call set (the ~39 legacy shims grow to match), or (b) thin `ai/` to
route through the narrow public facades / a subprocess LSP. **This handoff is a
PYTHON-RETIRE blocker, not a no-op** — the ai *port* is n/a, but the ai
*re-pointing* is real work owned by API-PYO3.

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

# Current Rust architecture

> Snapshot of the Rust workspace as of the **ARCH0–ARCH9**
> crate-and-registry cleanup. Use this page when picking up a new
> chunk; cross-reference [`docs/rust-rewrite.md`](../../rust-rewrite.md)
> for the long-form policy and the chunk log.

## Crate graph

```
                  +--------------+
                  |  tcl-lexer   |   no deps; spans, tokens, source map
                  +--------------+
                          ^
                          |
                  +--------------+
                  | tcl-registry |   command facts, taint metadata,
                  +--------------+   typed hook IDs, command forms
                          ^
            +-------------+------+-------------+
            |                    |             |
   +---------------+   +---------------+   +-----------------+
   | tcl-compiler  |   | tcl-lsp-core  |   | tcl-lsp-server  |
   |  IR/CFG/SSA   |   | folding,      |   | tower-lsp       |
   |  analyses,    |   | symbols,      |   | binary; holds   |
   |  codegen      |   | diagnostics   |   | the query db    |
   |               |   |               |   | + async diags   |
   +---------------+   +---------------+   +-----------------+
            ^                  ^                 |
            |                  |                 v
            +---------+--------+        +-----------------+
                      |                 |  tcl-lsp-db     |  salsa 0.26 query
                      +-----------------|  inputs +       |  database (incremental
                                        |  tracked queries|  analysis foundation)
                                        +-----------------+
                          ^
                          |
                  +--------------+
                  |  tcl-lsp-py  |   public PyO3 binding crate
                  +--------------+   (cdylib + rlib)
                          ^
                          |
                  +--------------+
                  | tcl-lsp-rust |   transitional alias — re-exports
                  +--------------+   tcl-lsp-py under the legacy
                                     `tcl_lsp_rust` Python module
                                     name; retires in vNext.
```

The arrows are dependency direction (consumer → provider). `tcl-lsp-
core` and `tcl-compiler` link against `tcl-registry` directly; the
PyO3 binding crates only exist to translate Python ↔ Rust shapes.

### Ownership rules

- **No `pyo3` in pure crates.** `tcl-lexer`, `tcl-registry`,
  `tcl-compiler`, `tcl-lsp-core`, and the future `tcl-lsp-server`
  must not depend on `pyo3`. The dependency graph above is the
  enforcement mechanism — any chunk that adds `pyo3` to a pure
  crate is rejected.
- **No product behaviour in PyO3 crates.** `tcl-lsp-rust` may only
  contain conversion code and Python-API back-compat shims. LSP
  feature providers, compiler passes, and registry queries live in
  `tcl-lsp-core`, `tcl-compiler`, and `tcl-registry` respectively.
- **No command-name tables outside `tcl-registry`.** Compiler,
  analyser, diagnostics, and LSP code ask the registry "which
  hook?" / "is this a taint source?" / "does this command have
  `-normalized`?". Adding a new command to the registry is the
  only place that code knows about command-specific facts.
- **Typed hook IDs.** Lowering and codegen specialisation is
  selected by a typed enum (`LoweringHookId`, `CodegenHookId`) on
  the matched `CommandSpec` / `SubCommand`. The compiler-side
  dispatcher matches exhaustively on the enum, so a new variant
  produces a deliberate compile-time error.

## Authoritative Rust paths

These paths are the canonical implementation; the corresponding
Python is either retired or kept only as a one-release fallback.

| Surface | Crate | Module | Status |
|---|---|---|---|
| Backslash substitution | `tcl-lexer` | `substitution` | authoritative |
| Tokeniser | `tcl-lexer` | `lexer` / `tokens` | authoritative |
| Spans / line index / source map | `tcl-lexer` | `span` / `line_index` / `source_map` | authoritative |
| Command registry & lookups | `tcl-registry` | `registry` / `commands/` | authoritative |
| Typed hook IDs | `tcl-registry` | `hooks` | authoritative |
| Command / subcommand forms | `tcl-registry` | `forms` | authoritative |
| Taint source / sanitiser facts | `tcl-registry` | `taint` | authoritative |
| Lowering hook dispatch | `tcl-compiler` | `lowering_hooks` | authoritative |
| Codegen hook dispatch | `tcl-compiler` | `codegen::emitter::bytecoded` | authoritative |
| IR / CFG / SSA | `tcl-compiler` | `ir` / `cfg` / `ssa` | authoritative |
| Analyser | `tcl-compiler` | `analyser` | default-on Python-supplemented |
| Folding ranges | `tcl-lsp-core` | `folding` | authoritative (Python wraps via shim) |
| Document symbols | `tcl-lsp-core` | `document_symbols` | authoritative (Python wraps via shim; native server wires the same provider) |

## Default-on, Python-supplemented paths

These chunks landed default-on through the env-var gate. Python
remains as a safety-net fallback for one release cycle, after which
the env var inverts to opt-out and the Python implementation
retires.

| Subsystem | Env var | Python fallback module | Notes |
|---|---|---|---|
| Background signature scan | `TCL_LSP_RUST_SIGNATURE_SCAN` | `core/analysis/signature_scan.py` | flipped in C40-default-on |

The **single-pass analyser has no env-var gate** any more. The Python
override it was meant to flip (`_merge_rust_with_python_supplement` in
`core/analysis/_analyser/__init__.py`) was deleted at #241 when that
module shrank to a ~47-LOC passthrough, so there is nothing left to
flip — `TCL_LSP_RUST_ANALYSER` does not exist in the tree. The native
`tcl-lsp-server` calls `Analyser::analyse()` directly
(`publish_analyser_diagnostics`), so analyser precision fixes are
already user-facing on the Rust path. Only four `TCL_LSP_RUST_*` gates
are live in the Python tree today: `…_SIGNATURE_SCAN` (above) plus
`…_OPTIMISER`, `…_INTERPROC`, and `…_GVN` (default-off shims, next
section).

## Default-off Rust shims

These chunks are feature-complete in Rust but still default to the
Python implementation. They flip to default-on once differential
parity has baked.

| Subsystem | Env var | Python module |
|---|---|---|
| Optimiser pass manager | `TCL_LSP_RUST_OPTIMISER` | `core/compiler/optimiser/_manager.py` |
| Interprocedural analysis | `TCL_LSP_RUST_INTERPROC` | `core/compiler/interprocedural.py` |
| GVN | `TCL_LSP_RUST_GVN` | `core/compiler/gvn.py` |

## Python fallbacks planned for deletion

After each chunk's env var has been default-on for one release
cycle, the Python fallback retires. Folding is the first LSP
feature with a pure-Rust home (`tcl-lsp-core::folding`); the Python
side now imports `tcl_lsp_rust.folding_ranges` and only retains the
`_normalise_overlaps` post-pass plus a fallback path for installs
without the wheel.

The full retirement list lives in the chunk log
([`docs/rust-rewrite.md`](../../rust-rewrite.md)) under the
**PYTHON-RETIRE** chunk; the v2.0 release deletes `core/`, `lsp/`,
`vm/`, `debugger/`, `explorer/`, `ai/`, and `scripts/` once every
chunk above has flipped.

## Crate boundary intentions

- **`tcl-lsp-core`** — pure LSP feature providers (folding,
  document symbols). No `pyo3`. Both the native LSP server and the
  PyO3 binding link against this crate so the algorithm has one
  canonical home.
- **`tcl-lsp-server`** — `tower-lsp` binary serving folding
  ranges and document symbols over stdio. ARCH8 lands the
  bootstrap; `S-document-symbols` adds the second provider; future
  `S*` chunks extend the set further (hover, completion, semantic
  tokens, diagnostics).
- **`tcl-lsp-py`** — canonical public PyO3 binding crate. ARCH9
  lands it as the new home for every `#[pyclass]` / `#[pyfunction]`
  surface.
- **`tcl-lsp-rust`** — transitional alias. Re-exports `tcl-lsp-py`
  under the legacy `tcl_lsp_rust` Python module name for one
  release cycle, then retires in vNext. Carries no product logic;
  new bindings land in `tcl-lsp-py`.

New LSP features land in `tcl-lsp-core`; any Python-facing wiring
lives in a thin per-feature `*_binding.rs` file under
`tcl-lsp-py/src/features/` that re-exports via `#[pyfunction]`.

## LSP server runtime — query database + async diagnostics

The native server (`tcl-lsp-server`) is no longer a bag of hand-maintained
caches. Its runtime state is:

- **`tcl-lsp-db` (salsa 0.26) query database.** `Backend` holds an
  `Arc<Mutex<TclDatabase>>`. Inputs: per-URI `SourceFile { text, dialect }` and
  a shared `AnalyserConfig { disabled_diagnostics, non_ascii_mode }`. Tracked
  queries wrap the existing sync providers: `file_analysis` (→
  `Arc<AnalysisResult>`), `document_symbols`, `semantic_tokens`, `folding_ranges`.
  The static command registry is carried as a durable field on the db (read via
  the `TclDb` trait), not a salsa input, so queries needn't make
  `CommandRegistry: PartialEq`. `AnalysisResult` derives `PartialEq` for salsa's
  no-unsafe `Update`/early-cutoff (the workspace forbids `unsafe`).
- **Caches eliminated.** The former `analyses`, `hover_cache`,
  `semantic_tokens_cache`, and `dialect_registries` maps are gone — replaced by
  the query graph + the durable registry. `did_open`/`did_change` set the
  `SourceFile` input once (the single invalidation point); reads clone the db
  handle onto a `spawn_blocking` worker and catch `salsa::Cancelled`. Only
  `workspace_index` (cross-document aggregate + disk scan) remains a manual
  structure for now.
- **Diagnostics are async + debounced.** `did_open`/`did_change` call
  `schedule_diagnostics`, which spawns the work after a 50 ms debounce (a burst
  of edits collapses to one run via a per-URI generation check) and returns
  immediately — the message loop is never blocked on analysis. The diagnostics
  path computes its base analysis with a *direct* `Analyser::analyse` (not the
  salsa query) on a blocking worker: salsa's `set_text` takes global write
  exclusivity, so a diagnostic read-handle held across the uncancellable analyse
  would block the next edit's write and stall worker threads. Interactive read
  handlers still use the memoised queries.
- **Shared CompilationUnit.** `lift_compiler_diagnostics` builds one
  `CompilationUnit` and runs both `run_all_checks` and `optimiser::optimise_unit`
  over it (no double lowering).

Measured impact (practcl.tcl, 8463 lines): post-edit interactive latency
~1080 ms → **1–20 ms** (async diagnostics); documentSymbol across a 119-file
corpus ~8.2 s → **43 ms** (Python → memoised Rust); diagnostics ~1.4 s → ~1.3 s
(shared CU). The remaining ~1 s diagnostic cost is the whole-file analyser walk,
which **per-item incremental analysis** targets — see
[`incremental-analysis.md`](incremental-analysis.md).

## Where to add a new fact

| Fact | Home |
|---|---|
| New command | `tcl-registry/src/commands/<dialect>/<name>.rs` |
| Lowering specialisation | new `LoweringHookId` variant + arm in `tcl_compiler::lowering_hooks::dispatch_lowering_hook` |
| Codegen specialisation | new `CodegenHookId` variant + arm in `tcl_compiler::codegen::emitter::bytecoded::dispatch_codegen_hook` |
| Taint source (top-level command) | stamp `Traits::TAINT_SOURCE` on the matching `CommandSpec` |
| Taint source (subcommand-shaped, e.g. `chan gets`) | stamp `Traits::TAINT_SOURCE` on the matching `SubCommand` |
| iRules option-driven check | declare the option in the registry (`OptionSpec`); consumer reads `spec.options` |
| Side-effect summary | populate `side_effects: &[SideEffect { ... }]` on the spec |

## Related

- [`lsp-performance.md`](lsp-performance.md) — measured Python-vs-Rust results,
  the optimisations shipped, and how to reproduce every number.
- [`incremental-analysis.md`](incremental-analysis.md) — the per-item
  incremental-analysis design, query graph, cascade, and staged plan.
- [`incremental-analysis-experiments.md`](incremental-analysis-experiments.md) —
  corpus, every Phase-0 experiment, discoveries, results snapshot, and the
  reasoning behind the plan.
- [`target-architecture.md`](target-architecture.md) — the zero-copy /
  single-parse / MVCC destination (salsa engine decision recorded).
- [`docs/rust-rewrite.md`](../../rust-rewrite.md) — chunking
  strategy, principles, chunk log.
- [`docs/kcs/kcs-qa-rust-shim-env-vars.md`](../../kcs/kcs-qa-rust-shim-env-vars.md) —
  Rust shim env-var reference.
- [`docs/rust-rewrite-test-audit.md`](../../rust-rewrite-test-audit.md)
  — test-port classification.

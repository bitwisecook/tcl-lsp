# Incremental analysis — per-item walk with cascade invalidation

> Companions: the full experiment lab notebook (corpus, every experiment,
> discoveries, and the reasoning that produced this plan) is in
> [`incremental-analysis-experiments.md`](incremental-analysis-experiments.md);
> the shipped perf work and measurements are in
> [`lsp-performance.md`](lsp-performance.md); the runtime model in
> [`current-architecture.md`](current-architecture.md).

> **Phase-0 experiment findings** (`rust/tcl-compiler/examples/incr_experiments.rs`,
> over tcl8.6 + tcllib corpus):
> - **E3 — GO.** On practcl.tcl the per-proc *walk* is ~82% of `analyse`
>   (~785 ms) and the cross-item *tail* (W123/arity/interproc) is ~18%
>   (~167 ms). Per-item recompute attacks the dominant 82%; the unavoidable
>   per-edit aggregate floor is ~167 ms (itself incrementalisable later).
> - **E1 — 98.9% item-local.** Inserting a benign comment in the last proc
>   leaves every *preceding* item byte-identical in 345/349 files. The **4
>   exceptions are all global-variable diagnostics** (W210 read-before-set /
>   W211 set-but-unused) that span procs — a real, narrow cross-item class.
> - **E2 — GO (100%).** Prepending K blank lines and un-shifting every fact by
>   K reproduces the original `AnalysisResult` for 598/598 corpus files — the
>   analyser is fully offset-shift-invariant, so per-item facts produced at
>   offset 0 rebase exactly. (Validates the slice-2 relative-offset model.)
> - **E6 — GO.** On practcl: CompilationUnit build ~118 ms / 115 procs ≈ 1 ms
>   per proc; the interproc `pure`/effects fixpoint is only ~10.6 ms; per-proc
>   lattice recompute + interproc cascade is cheap.
> - **E7 — GO.** `optimise_with_dialect` rebuilds its own CU+interproc (~129 ms
>   redundant); sharing one CompilationUnit with `run_all_checks` recovers it.
> - **E4/E8 — GO.** A salsa event-counter prototype (`tcl-lsp-db/tests/
>   early_cutoff.rs`) confirms the cascade mechanism: a *body* edit re-executes
>   exactly **one** `item_analysis` (the unchanged sibling is reused); a
>   *signature* edit re-executes the cross-item aggregate but **zero** item
>   bodies (field-level deps + the caller→callee-sig edge); and a body change
>   that leaves the item's *output* unchanged cuts off the aggregate
>   (tracked-output early cutoff). **Finding:** salsa input setters always bump
>   the revision (no value-equality cutoff on inputs), so the per-item build
>   must set an item's body input **only when the item-tree diff says it
>   changed** — otherwise every keystroke re-runs direct dependents.
> - **E5 — FOUND A GAP (the fuzzer works).** The differential fuzzer
>   (`tests/differential_incremental.rs`, `incremental == fresh` under random
>   edits) shows `analyse_incremental` diverges from `analyse`. Root cause: even
>   *unedited*, `analyse(src) != analyse_commands(src, segment(src), true)` —
>   **`all_procs` match, but the diagnostic sets differ**. So the existing
>   pre-segmented entry point is not diagnostic-equivalent to `analyse`. The
>   structural analysis is already incremental-sound; the per-item build's job is
>   to make **diagnostic emission byte-identical to `analyse`**, with the fuzzer
>   as the gate and the full-rebuild fallback for anything it can't match.
> - **Implication / decision.** The per-item firewall is sound for ~99% of
>   edits, but **global-variable usage is a cross-item fact** (W210/W211 on
>   globals span procs). **Decision: detect cross-proc global-var flow and fall
>   back to a full rebuild for those files** (~1%) rather than model a
>   `global_var_usage` aggregate — simpler, and correctness is unconditional via
>   the fallback. Item-locality is a *perf* heuristic; the correctness contract
>   is the **fuzzer + full-rebuild fallback** below.

> Design for making the Tcl analyser incremental at *item* granularity, so a
> keystroke inside one proc recomputes that proc, not the whole file. Grounded
> in two data-flow audits (analyser walk; IR/lattice pipeline). Companion to
> [`target-architecture.md`](target-architecture.md) (Layer 4, the cascade).

## Why

`Analyser::analyse` is linear (~0.14 ms/line) but whole-file: an 8.5k-line file
costs ~1 s per edit. Measured split per edit on practcl.tcl:

| stage | cost | granularity today |
|---|--:|---|
| lex + green tree + segment | ~3 ms | whole-file (cheap) |
| **analyser walk (scope trees + diagnostics)** | **~1015 ms** | **whole-file** |
| CompilationUnit + `run_all_checks` | ~146 ms | per-`FunctionUnit`, built whole-file |
| `optimise_with_dialect` (2nd CompilationUnit) | ~141 ms | per-`FunctionUnit`, built whole-file |

The walk dominates and `analyse_incremental` does **not** help — it reuses
segmentation (3 ms) but re-walks every command. Parsing is already negligible.
The lever is making the walk and the lattices recompute **only the edited item**.

## The firewall: signatures vs bodies

The audits show the split that makes this tractable:

- A **proc/method body** is **item-local**: its `param_traits`, local command
  invocations, local diagnostics, and scope subtree are a pure function of
  *(body text, params, enclosing namespace, registry, dialect, stub overlay)* —
  nothing from sibling bodies (analyser audit §3B, §4).
- The **cross-item facts are signatures**: name resolution and the W123
  unresolved-command / arity passes read the file's *set* of
  `all_procs ∪ all_classes ∪ command_aliases ∪ ensemble_namespaces` and the
  namespace tree — i.e. item **headers**, not bodies (analyser audit §3A, §3C).
- The **lattices** are already per-`FunctionUnit` (SSA→def-use→SCCP→type→
  rendered→taint, GVN); the only genuine cross-item cascade is the
  interprocedural `pure`/effect fixpoint and the taint re-run that consumes it
  (lattice audit §4).

So the query graph keys **bodies** (expensive, item-local) separately from
**signatures** (cheap, cross-item). A whitespace or in-body edit changes one
body and leaves every signature — and therefore every other item's analysis,
the resolution table, and the interproc summary — untouched.

## Query graph

```
input    source(file), dialect(file), config(global), registry(static)
derived  tokens(file)        ← source                      # ~3ms, whole-file
derived  item_tree(file)     ← tokens                      # FIREWALL
derived  item_sig(item)      ← item_tree(file)             # header: name, params,
                                                           #   namespace, span, kind
derived  file_decls(file)    ← item_sig(item)*             # all_procs/classes/
                                                           #   aliases/ensembles + ns tree
derived  item_body(item)     ← item_tree(file)             # body token slice (rel offsets)
derived  item_analysis(item) ← item_body(item), file_decls(file), registry, config
                                                           # the expensive per-line walk,
                                                           #   bounded to one item; emits
                                                           #   RELATIVE-offset facts
derived  file_analysis(file) ← item_analysis(item)* , file_decls(file)
                                                           # rebase rel→abs, aggregate,
                                                           #   run W123 / arity over the union
# lattices (per FunctionUnit), cascade layer:
derived  cfg(item)           ← item_body(item)
derived  ssa/sccp/type/rendered/taint_intra(item) ← cfg(item), registry
derived  interproc(file)     ← item_sig(item)*, call_graph(file)   # pure/effects fixpoint
derived  taint(item)         ← taint_intra(item), interproc(file)  # cross-item re-run
derived  diagnostics(file)   ← file_analysis(file), checks(item)*, optimiser(item)*
```

**Early cutoff is the point.** `item_tree`/`item_sig` are keyed on *structure*,
so a body-only edit produces equal sigs → `file_decls`, `interproc`, and every
other `item_analysis` are reused; only the edited `item_analysis` + its lattices
recompute. A signature edit (rename/param change) changes `item_sig` → cascades
to `file_decls` (re-resolve call sites, re-run W123) and `interproc` (re-run
taint on callers) — exactly the dependents, nothing more.

## The load-bearing refactor: relative-offset item analysis

`item_analysis(item)` must be memoisable by *body text*, so an unmoved-but-
shifted proc (lines inserted above it) is a cache hit. Therefore the per-item
walk must emit **offsets relative to the item's start**, and `file_analysis`
**rebases** them to absolute positions using `item_sig.span.start`. This mirrors
rust-analyzer's `ItemTree` (position-independent) + per-body lowering.

Concretely (analyser audit §4), `analyse_commands` (`state.rs:773`) and the
`process_command` dispatch must stop mutating `self.result`/`self.const_strings`
directly and instead take the cross-item context as **inputs** and return the
item's facts as **outputs**:

- inputs: `body_text`, `params`, `namespace`, `&file_decls`, `registry`, `config`
- outputs: `param_traits`, `local_invocations` (rel spans), `local_diagnostics`
  (rel spans), `scope_subtree`, `const_strings`, `var_reads`

The order-sensitive file-global facts the audit flagged — `command_aliases`,
`ensemble_namespaces`, the namespace tree — are **top-level** concerns, so they
live in `file_decls` (computed from `item_sig`s in source order), not inside
body analysis. Bodies read them read-only.

## Cancellation

For diagnostics to rejoin the shared salsa graph (instead of the current direct
`Analyser::analyse` the server runs detached to dodge write contention), the
per-item walk must check `db.unwind_if_cancelled()` at command boundaries so a
new edit cancels an in-flight analysis quickly — the property
`Analyser::analyse` lacks today, which forced the decoupling in
`run_diagnostics_core`.

## Correctness guard

Every step must produce **byte-identical** `AnalysisResult` / diagnostics to the
current whole-file walk. Guards, in order:
1. the differential corpus (`rust/tcl-compiler/tests/differential_*`) and the
   analyser unit tests — pin exact diagnostics/spans;
2. `make test-lsp-e2e-rust` — observable parity;
3. a new property test: for random edits, `incremental == fresh` (the existing
   `test_edit_tracking_stress_e2e` already asserts this for tokens/symbols;
   extend to diagnostics).
When incremental can't prove equivalence (error recovery, stub overlays — the
existing `analyse_incremental` guard), fall back to a full walk.

## Staging (each slice behaviour-preserving, differential-guarded)

1. **`item_tree` + `item_sig` + `file_decls`** as salsa queries — additive,
   no behaviour change; assert `file_decls` matches the current decl scan.
2. **Relative-offset item body analysis** — refactor `analyse_commands` to the
   pure inputs→outputs shape above, returning rel-offset facts; `file_analysis`
   rebases + aggregates. Prove byte-identical to the whole-file walk on the
   corpus. (This is the bulk.)
3. **Memoise `item_analysis`** in salsa keyed on `item_body` — the perf win:
   body edit ⇒ one item recomputes. Measure heavy-edit drop.
4. **Per-item lattices + `interproc` cascade** — key SSA/SCCP/type/taint per
   item; `interproc` fixpoint over `item_sig`s; taint re-run only on affected.
5. **Cancellation-aware walk** — `unwind_if_cancelled` at command boundaries;
   move diagnostics back onto the shared query graph (retire the direct-analyse
   detour) and delete the residual `document_analysis_gate`.

## Verification

`cargo test --workspace` (incl. differential corpus), `make test-lsp-e2e-rust`,
and `scripts/dev/bench_lsp_backends.py` heavy-edit (target: single-proc edit on
practcl.tcl from ~1 s to low-ms).

## Implementation status

| Piece | State |
|---|---|
| salsa query DB (`tcl-lsp-db`): inputs + `file_analysis`/`document_symbols`/`semantic_tokens`/`folding_ranges` | **shipped** |
| Server caches → queries (`analyses`/`hover`/`semantic_tokens`/`dialect_registries` deleted) | **shipped** |
| Async + debounced diagnostics (heavy-edit fix) | **shipped** |
| Shared CompilationUnit (E7) — `optimiser::optimise_unit` | **shipped** |
| Phase-0 experiments E1–E8 + differential fuzzer + cascade prototype | **shipped** (this doc's findings) |
| Slice 1 — `item_tree`/`item_sig`/`file_decls` | **shipped** (corpus-gated) |
| Slice 2 — per-item walk via deferred bodies (`analyse_per_item`); `incremental == fresh` fuzzer to 0 | **shipped** (byte-identical over corpus) |
| Slice 3 — memoise per-body analysis (`item_body_analysis`); offset-invariant keys | **shipped** (firewall + offset-invariance tests) |
| Slice 4 — per-procedure lattice memoisation (`build_for_memoized` + `lattice_rebase`), offset-invariant, wired into `file_analysis_incremental` via an idempotent content cache | **shipped** (byte-identical; e2e parity) |
| Slice 5 — diagnostics on the cancellable salsa graph (`file_analysis_incremental`), coalescing per-URI worker (CPU-stress-robust), `document_analysis_gate` retired | **shipped** (e2e parity; edit-storm stress green) |

> **Remaining (future):** the interprocedural taint cascade still re-runs
> `propagate_taints` for every function on each edit (the per-function
> baseline lattices are reused; only the taint re-run is whole-file); the
> optimiser's second `CompilationUnit` (`lift_compiler_diagnostics`) is still
> whole-file. A salsa-native per-item lattice graph (interning post-inline IR)
> would replace the process-wide content cache but needs `Hash`/`Eq` across the
> IR graph (blocked today by float fields / no `serde`).

## How to run the experiments

- **E1/E2/E3/E6/E7** (item-locality, offset-invariance, cost split, lattice
  costs, shared-CU saving): `cargo run --release -p tcl-compiler --example
  incr_experiments` (reads the `tmp/` corpus).
- **E4/E8** (salsa early-cutoff + cascade breadth): `cargo test -p tcl-lsp-db
  --test early_cutoff`.
- **E5** (the differential fuzzer, `incremental == fresh`): `cargo test -p
  tcl-compiler --test differential_incremental -- --ignored` (corpus-gated,
  slow). Currently surfaces the pre-existing `analyse` vs `analyse_commands`
  diagnostic inconsistency — slice 2 must close it; this test is the gate.

## Slice-1 execution notes (for whoever picks it up)

The non-trivial part of `item_tree` is reproducing the analyser's
**namespace-qualified** item detection without divergence. The detection lives
in the stateful scope walk:

- `handle_proc_command` (`analyser/handlers.rs:292`): `name = args[0]`,
  body span = `arg_tokens[2].span`, qualified via
  `namespace_from_scope_path(scope_path)` + `qualify(ns, name)`
  (`handlers.rs:65`, currently `pub(super)` — expose it).
- `handle_namespace_eval_command` (`handlers.rs:477`) creates the child scope
  whose path drives qualification; `oo::class create` /`oo::define`
  (`handlers.rs:1154,1229`) for classes/methods; `interp alias` /`namespace
  ensemble` for the alias/ensemble sets in `file_decls`.
- Bodies can define **nested/global procs** (E1's perf-file finding) — the
  extractor must recurse into bodies, and a body edit that adds/removes a nested
  def is a *signature* change (cascade), not a pure body edit.

The safest path is to derive `item_tree` from the **CST** the analyser already
builds (so detection can't diverge), gated by a corpus test asserting
`file_decls`'s proc/class/alias/ensemble set equals the current
`AnalysisResult.all_procs`/`all_classes`/`command_aliases`/ensembles. Do not
ship slice 1 until that test is green across the corpus.

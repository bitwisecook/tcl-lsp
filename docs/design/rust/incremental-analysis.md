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
| Slice 4 — per-procedure lattice memoisation (`build_for_memoized` + `lattice_rebase`), offset-invariant | **shipped** (byte-identical; e2e parity) |
| Slice 5 — diagnostics on the cancellable salsa graph (`file_analysis_incremental`), coalescing per-URI worker (CPU-stress-robust), `document_analysis_gate` retired | **shipped** (e2e parity; edit-storm stress green) |
| Salsa-native per-procedure lattice graph (`function_lattice` query interning each proc's offset-0 body + `CfgContext`); both consumers (analyser tail + optimiser via `compiler_check_diagnostics`) on salsa; process-wide content cache retired | **shipped** (byte-identical; offset-invariant firewall + e2e parity) |
| Per-item walk Phase B — widen the fast path: qualified `$::g` reads captured + replayed on the shell's globals at graft; the enclosing-context fallback narrowed from "any namespace/global/`variable`/qualified-read body" to **duplicate definitions** (top-level + nested self-redefinition) and **class-defining / qualified-writer bodies** | **shipped** (≈42% of corpus files now take the incremental fast path, up from the qualified-read-trigger floor; byte-identical — corpus + fuzzer gated) |
| Per-item walk Phase C — widen the fast path to the enclosing-variable class: the three residual divergences made byte-identical (W120 anchored at the source-earliest invocation; W002 disabled-command shadow check deferred to the tail over the merged `all_procs`; W304 `$var` resolution deferred to the tail over the full source); qualified-read replay gated on source-order visibility; `body_needs_enclosing_context` narrowed to just `variable ns::x`; post-walk `E…` syntax-error backstop added (mirrors `analyse_incremental`) | **shipped** (≈42%→**86.6%** of corpus files take the fast path; byte-identical — `per_item_corpus` *and* the new `per_item_matches_analyse_under_edits` edit fuzzer + `differential_incremental` + e2e gated) |

> **Phase B — what the fast path now covers, and what still falls back.**
> The per-item walk's enclosing-context fallback (`body_needs_enclosing_context`)
> used to fire on *any* body touching namespace/global/`variable` state or a
> qualified `$::g` read — i.e. almost every non-trivial file.  Phase B:
>
> - **Captures + replays qualified reads.** A body's `$::g` read is recorded
>   (`Analyser::capture_global_reads`) when it misses the isolated body's empty
>   global scope, then replayed on the shell's real `::g` at graft — so a body
>   that only *reads* enclosing globals no longer needs the fallback.
> - **Narrows the fallback** to the two patterns the isolated-body decomposition
>   genuinely can't reproduce byte-for-byte: (1) **duplicate definitions** — the
>   same proc/method qualified name defined twice (platform-conditional `proc`s
>   *and* the lazy-init self-redefinition `proc p {a} { …; proc p {a} {real} }`,
>   detected by tracking every defined proc name across the shell + body
>   fragments); (2) **class-defining or qualified-writer bodies** (`oo::class` /
>   `oo::define`, or `set ::ns::x`) — whose `all_variables` / class-method facts
>   accumulate across bodies in a whole-file walk but only *merge* in the graft.
>
> Result: ~42% of corpus files take the incremental fast path (a body edit
> recomputes one body + shell + tail).  The remaining ~58% — those that
> **declare/write** enclosing variables (`variable` / `global` / `set ::x`) or
> define classes in a body — still fell back (addressed in **Phase C** below).

> **Phase C — the enclosing-variable / class class joins the fast path
> (shipped).**  A probe (`tcl-compiler/examples/per_item_divergence`,
> analyse-vs-per-item over the `tmp/` corpus with the enclosing-context fallback
> bypassed) showed the broad Phase-B fallback was ~98.6% unnecessary: bypassing
> it left only **three** diagnostic-divergence classes and two book-keeping nits,
> all narrow and reproducible.  Each was made byte-identical at the
> isolated-body / graft / tail seam — the rust-analyzer "cross-item facts live in
> the aggregate, not the body" split — so the fallback could be narrowed to
> almost nothing:
>
> - **W120** (missing `package require`) now anchors at each command's
>   *source-earliest* invocation instead of the first in walk order, making it
>   independent of whole-file-DFS vs per-item shell-order (it is emitted before
>   `canonicalize_result_order`).  Applies to both paths — no behaviour change for
>   `analyse` beyond determinism.
> - **W002** (disabled-in-dialect command) — its user-proc-shadowing suppression
>   reads the file's whole `all_procs`, which an isolated body lacks.  The body now
>   *captures* would-be-W002 sites (`pending_disabled_commands`) and the tail
>   re-applies the shadow check against the merged `all_procs`
>   (`flush_disabled_command_diagnostics`) — mirroring `pending_arity` /
>   `capture_global_reads`.
> - **W304** (missing `--`) — only its `$var` branch is source-dependent
>   (`last_literal_set_value_for_var` scans the *whole* source for the most-recent
>   literal `set`).  That branch is deferred (`pending_w304`) and classified in the
>   tail where `self.source` is the full file; every other W304 branch stays
>   inline.
> - **Qualified-read replay** is now gated on source-order visibility: a captured
>   `$::ns::v` is replayed only when its enclosing definition *precedes the body*
>   (a whole-file DFS walks a proc body before a later top-level `set ::ns::v`, so
>   it records no reference there).
> - **`body_needs_enclosing_context`** is narrowed from "any
>   `variable`/`global`/`set ::x`/class body" to just **`variable ns::x`** (a
>   `variable` linking to a *sub-namespace* variable — the lone residual whose
>   merged `warn_if_unused` / `definition_span` is walk-order-sensitive).  Plain
>   `variable x`, `global`, `upvar`, qualified writes, and class definitions all
>   take the fast path.
> - A post-walk **`E…` syntax-error backstop** (mirroring `analyse_incremental`)
>   re-analyses fully whenever the per-item result carries any error diagnostic —
>   locally-unbalanced braces that pass `script_is_complete` engage `analyse`'s
>   error-recovery machinery, which the per-item walk does not reproduce.
>
> Result: **86.6%** of corpus files take the fast path (up from ~42%), 100%
> byte-identical; large *non-duplicate* files (e.g. `pki.tcl`, 3.3 kLOC) drop from
> ~360 ms full-analyse to ~115 ms per warm edit.  Gated by the unedited
> `per_item_corpus` test **and** a new edit fuzzer
> (`per_item_matches_analyse_under_edits`, the `incremental == fresh` contract for
> the per-item walk under random edits — which the older `differential_incremental`
> does not cover, as it exercises `analyse_commands`-based `analyse_incremental`).
>
> **Where the remaining per-edit time actually goes (post-Phase-C profiling).**
> Phase-timed on the salsa graph (warm DB, single-char body edit), the picture
> that the original "memoise the tail" sub-tasks assumed turns out **not** to
> hold — the tail's *emission* is already cheap; the cost is the unit **build**:
>
> - **The analyser tail's `emit_cfg_ssa_diagnostics` emission is ~6 ms** once it
>   consumes the memoised `cu_override`; the 130 ms it costs on the *no-override*
>   path (`analyse_per_item`) is entirely the **`CompilationUnit` build** it does
>   for itself.  Every other tail emitter (W123 / arity / W120 / var-usage / W002
>   / W304) is ~0 ms.  So *making the emission incremental buys almost nothing.*
> - The dominant per-edit cost on a fast-path file is the **`CompilationUnit`
>   build** — `memoised_compilation_unit`, run once in *each* of the two tracked
>   queries (`file_analysis_incremental` + `compiler_check_diagnostics`).  On
>   `parse_lemon.tcl` (7.4 kLOC, 177 fns): `build_for_memoized` ≈ 80 ms +
>   `with_interprocedural` ≈ 19 ms, ×2 queries.  The per-function *lattices* are
>   memoised (cache hits across edits), so this 80 ms is the **non-memoised
>   whole-module work**: `lower_to_ir` + module `build_cfg` +
>   `collect_call_site_constants` + the 177 lattice clone/rebases — i.e. O(file)
>   lowering, the genuine floor.  `run_all_checks` (≈ 23 ms) + `optimise_unit`
>   (≈ 13 ms) are minor by comparison.
> - **Net:** fast-path large files already sit at the *target* — `parse_lemon`
>   ≈ 150 ms (`file_analysis_incremental`) + 141 ms (`compiler_check`); `pki.tcl`
>   (3.3 kLOC) ≈ 115 + 119 ms.  Driving them lower means attacking the
>   lowering/CFG floor (incremental lexing/lowering, or sharing one built unit
>   between the two queries when their `LexerConfig`s coincide — they differ only
>   for tcl8.4 / iRules) — a larger architecture step, *not* the diagnostic-tail
>   memoisation the original plan named.
>
> **Shared `CompilationUnit` build (shipped).** The unit was built twice per
> edit (once per diagnostics query).  A tracked `compilation_unit(file, cfg)`
> query keyed on an interned `LexerCfgKey` lets the analyser tail
> (`file_analysis_incremental`) and the optimiser/compiler-checks
> (`compiler_check_diagnostics`) **share one build per edit** whenever their
> lexer configs coincide — every dialect but `tcl8.4` / `f5-irules`.  Combined
> per-edit latency on `parse_lemon.tcl` (7.4 kLOC) drops ~287 ms → ~202 ms (one
> build eliminated).  `CompilationUnit` gained `PartialEq` so salsa returns
> `Arc<CompilationUnit>`; gated by `compilation_unit_shared_across_consumers`.

> **Duplicate procs + multi-method classes on the fast path (shipped).** The
> remaining fallback census (`duplicate` 86, `variable_ns` 44, `proc_collision`
> 12, `syntax_error` 11) was dominated by two avoidable causes:
>
> - **Multi-method classes were a false duplicate** — method deferred-bodies had
>   an empty `scope_name`, so the duplicate detector treated every method after
>   the first as a duplicate of `(is_method, "", "")`; *any* class with 2+
>   methods fell back.  Methods now carry their qualified name, so distinct
>   methods are distinct (only a genuinely twice-defined method falls back).
> - **Genuine proc duplicates** now graft byte-identically: a whole-file
>   `define_var` unions each definition's locals with last-definition-wins on
>   shared keys, reproduced by overwrite-on-collision for body-owned
>   `all_variables` keys while param keys keep the shell's real definition span
>   (taking only the last body's references).
>
> The cross-item facts an isolated/in-place body can't reproduce are handled
> precisely (not by a blanket fallback): **object instances** (`[Cls new]` /
> `Cls create v`) are captured in the isolated proc body and replayed against the
> shell's full `all_classes` at graft (no memo-key change), with a per-method
> snapshot fallback for in-place method bodies; **class extension** (`oo::define`)
> falls back only on an `all_classes` key collision.  Result: fast-path coverage
> **86.6% → 92.2%**, 100% byte-identical, 0 diagnostic divergence.
>
> **Still not won.**
> - **The `variable ns::x` (`variable_ns`) + `syntax_error` fallbacks** remain
>   (correct, narrow).
> - **The `CompilationUnit` per-edit floor is architecture-level.** Phase-timed
>   on `parse_lemon.tcl` (7.4 kLOC, 177 procs), the per-edit CU rebuild splits
>   into: `lower_to_ir` ~26 ms + module `build_cfg` ~10 ms (both O(file)) + the
>   **per-procedure loop ~52 ms** + `with_interprocedural` ~19 ms.  The
>   per-procedure 52 ms is *not* lattice compute — the lattices are memoised
>   (`function_lattice`) — it is the **offset-invariant plumbing run for every
>   procedure on every edit**: clone the proc's offset-0 IR body + `rebase_script`
>   it, then deep-clone the memoised `FunctionUnit` + `rebase_function_unit` it to
>   the proc's real offset.  Because the rebase target (the offset) changes
>   whenever an edit shifts a procedure, the rebased unit can't be cached across
>   edits, so this plumbing is irreducible without either **incremental
>   lowering/CFG** or making the diagnostic consumers **offset-aware** (consume
>   offset-0 units + a per-proc offset, no rebase) — both large refactors.
>   - **`function_lattice` barely engages for real files.** Instrumenting the
>     proc loop shows *every* procedure in init/pki/parse_lemon/textutil takes the
>     **fresh-build** branch — `params_constants_from_call_sites` returns `Some` for
>     nearly all of them, and the memo was keyed *without* `param_constants` so
>     those bypass it.  So the 52 ms is fresh SSA/SCCP/type/taint builds, and the
>     headline lattice memo only helps callee-free procs.
>   - **`param_constants` folded into `FnLatticeKey` (shipped).** The
>     caller-uniform-literal SCCP seeds (`param_constants` /
>     `params_constants_from_call_sites`) are now part of the memo key, so a
>     procedure with them engages the `function_lattice` memo instead of
>     bypassing it via the old `param_constants.is_none()` guard.  The seeds are
>     carried through `LatticeRequest` in a deterministic, hashable,
>     position-independent encoding (`(param, version, string)`, sorted —
>     `encode_param_constants` / `decode_param_constants`), with a defensive
>     fresh-build fallback if a seed is ever not a string const (the only shape
>     the producer emits).  `function_lattice` decodes the seeds and builds via
>     `FunctionUnit::build_with_param_constants`, so a procedure rebuilds its
>     lattice only when a caller's literal at that position changes (a new key)
>     and is an offset-invariant cache hit otherwise.  With the memo now engaging
>     for nearly all procs, clone+rebase is **~9× cheaper than fresh-build** (the
>     measured-in-isolation figures: pki 4 ms vs 38 ms; parse_lemon 6 ms vs
>     52 ms).  On the noisy CI box the aggregate BOTH-queries win is partly
>     masked by the O(file) lowering floor that now dominates, but the achievable
>     floor drops (pki BOTH min-of-N **~175 ms → ~153 ms**, matching the original
>     ~150 ms experiment).  Byte-identical — the new
>     `function_lattice_memoises_param_constant_procs` db test proves a
>     param-constant callee now executes a `function_lattice` query and is reused
>     across an unrelated edit, and the full corpus differential
>     (`compiler_check_memo_matches_uncached_over_corpus`) + `differential_incremental`
>     + `per_item_corpus` + e2e all stay green.  This was previously tried and
>     reverted only because it *extended* the memo byte-identity bug below to more
>     procedures; re-landed on top of that fix (#616, see "Memo byte-identity
>     (fixed)").
> - **`practcl.tcl`** still falls back on a genuine twice-defined method-style
>   definer and is OO-heavy, so it needs that architectural step before it leaves
>   the full-rebuild path.

> **Memo byte-identity (fixed).**  The salsa-native lattice graph (#604) promises
> that the memoised `build_for_memoized` (offset-0 per-procedure `function_lattice`
> + rebase) is byte-identical to a fresh whole-module `build_for_with_config`.  A
> corpus differential (`tcl-lsp-db/tests/compiler_check_corpus.rs`, comparing
> `compiler_check_diagnostics` vs `compiler_check_diagnostics_uncached`) found
> ~30% of files diverging.  The root cause was **nondeterminism, not an
> offset-0-vs-whole-module analysis difference** — both builds disagreed run-to-run
> with themselves.  Five `HashMap`-iteration-order dependencies, each fixed to a
> stable order:
>
> 1. **`shimmer::span::phi_span`** picked the *first* incoming def span in
>    `phi.incoming` (a `HashMap`) → nondeterministic `S101` span; now the earliest
>    (min) span.
> 2. **`compiler_checks::run_all_checks`** emitted diagnostics in producer order
>    (per-function `HashMap` walks); now sorted on a total
>    `(span, code, category, severity, message, replacement)` key
>    (`sort_diagnostics`).
> 3. **`optimiser::optimise_unit`** allocated monotonic group ids and emitted in
>    `cu.procedures` / def-use-chain `HashMap` order; now canonicalised before
>    overlap arbitration (stable sort) with group ids renumbered by first
>    appearance (`renumber_groups`).
> 4. **`taint::find_destructive_file_warnings`** (W313) named the *first offending
>    path variable* from `ssa_stmt.uses` (a `HashMap`), so `file rename $a $b`
>    reported `$a` or `$b` at random; now iterates path variables in argument
>    (source) order (`arg_var_names_ordered`).
> 5. **`type_infer`** folded the order-sensitive `type_join` over a phi's
>    predecessors in `HashSet` order — and `type_join` records only a `(from, to)`
>    pair for a 3+-way shimmer, so the `S101` message named different types
>    run-to-run; the predecessor list is now sorted before the fold.
>
> With these, the memo and whole-module builds agree byte-for-byte across the full
> `tmp/` corpus (893 files), stable across process-level `HashMap` seeds.  The
> corpus differential is the (now-passing) regression guard, `#[ignore]`d only for
> being slow (`--ignored`, ~100 s).  This **unblocks extending the lattice memo**
> (the `param_constants` win above can now land on top).

> **OO method-body memoisation (shipped).** Method bodies were walked *in place*
> in pass 2 (not memoised), so a body edit re-walked every method.  They are now
> analysed as offset-0 isolated units and grafted like procs: `DeferredBody`
> carries the method's defining namespace + params + class instance variables,
> `analyse_proc_body_isolated` reconstructs a `ScopeKind::Method` scope (so
> `in_method` dispatch recording fires) with the instance variables pre-bound,
> and the proc/method pass-2 branches are unified through `body_fn` +
> `graft_proc_body`.  `ItemBodyKey` gains `is_method` + `class_variables`.  W308
> object tracking is the one method-specific subtlety — a method resolves the
> class against the whole file's classes, not analyse's DFS prefix, so a method
> that *actually* records an instance falls back (detected precisely: captured
> candidates are replayed at graft and the fallback fires only when
> `instance_classes` truly changed, so benign `dict create` stays fast).  Result:
> pt_rdengine_oo.tcl (2.2 kLOC, 113 methods) per-edit **165 ms → 44 ms**;
> cookiejar / disjointset / metaclass 8–16 ms.  Coverage holds at 92.2%,
> byte-identical; gated by `method_body_edit_recomputes_one_item` + the corpus +
> edit fuzzer.

> **Salsa-native lattice graph (shipped).** The per-procedure baseline lattices
> (CFG → SSA → def-use → SCCP → type → rendered → intra-procedural taint) are now
> memoised by the salsa-native `function_lattice` query, replacing the
> process-wide content cache (so salsa garbage-collects unreferenced entries).
> `build_for_memoized` normalises each procedure body to offset 0 and hands it
> (with the module's `CfgContext`, interned once per build) to a callback that
> interns the `FnLatticeKey` and demands `function_lattice`; the builder rebases
> the returned offset-0 unit to the procedure's real span (`lattice_rebase`).
> Reuses the same `build_cfg_function_with_upvars` call `build_cfg` makes per
> procedure, so the rebuilt unit equals the whole-module build's (modulo offset),
> under `db.registry` (byte-identical to both consumers' `build_default` +
> `load_dialect`).  The optimiser path was the architectural blocker — it ran
> *off* salsa with no db handle; it now runs through the `compiler_check_diagnostics`
> tracked query (server filters the master-switch / per-code disables + lifts), so
> both diagnostics consumers share the same memoised lattices.  Gated by the
> per-item corpus + `incremental == fresh` differential fuzzer + the db
> equivalence/offset-invariance tests (`function_lattice_reused_on_body_shift`,
> `compiler_check_diagnostics_matches_uncached`) + e2e parity.
>
> **Remaining (future).**
>
> - **Interprocedural taint cascade (shipped — backlog #1).**
>   `with_interprocedural` re-ran `propagate_taints` for every procedure every
>   edit.  A salsa-native `taint_cascade` query (`tcl-lsp-db`) now layers on
>   `function_lattice`'s offset-0 baseline so an unchanged procedure whose
>   reachable-callee summaries are unchanged reuses its cached taints.  The taint
>   lattice is `ValueKey`-keyed (span-free), so the cascade runs over the offset-0
>   baseline and installs the result directly into the rebased unit — **no rebase
>   needed**.  The key (`TaintSummaryKey`) captures exactly what `propagate_taints`
>   reads from the summary: the full procedure-name set (call-resolution domain)
>   plus the taint-relevant projection (`writes_global`, `calls`,
>   `return_passthrough_param`, `params`) of the cascade root + its transitive
>   callees — so a body edit that flips a reachable callee's passthrough /
>   global-write re-interns the key for exactly the callers that reach it.
>   `CompilationUnit::with_interprocedural_memoized` routes each procedure's taint
>   through a callback (top level stays fresh); the whole-module summary is still
>   built per edit (it is the memo's input), so the saving is only the
>   per-procedure re-run — **net-small, as predicted**.  Byte-identical: the
>   `compiler_check` corpus differential proved the minimal-summary reconstruction
>   cold over 893 files, and a new edit-staleness db test
>   (`taint_cascade_matches_uncached_under_edits`) proved no stale cache survives a
>   callee-behaviour edit.  Gated also by `taint_cascade_reused_on_unrelated_edit`
>   (one cascade recomputes per unrelated edit) + e2e.

## Scoping the per-edit lowering floor (backlog #3)

The `param_constants` fold (above) made `function_lattice` engage for nearly
every procedure, so the per-procedure *lattice compute* is now memoised and the
per-edit cost is dominated by the **non-memoised whole-module work** in
`build_for_inner` / `memoised_compilation_unit`.  Fresh phase-timing
(`cargo run --release -p tcl-lsp-db --example tail_profile FILE=…`, warm DB,
single-char body edit):

| file | LOC / fns | `file_analysis_incremental` | `compiler_check` | BOTH | CU build+interproc |
|---|---|--:|--:|--:|--:|
| `pki.tcl` | 3.3k / 75 | ~111 ms | ~133 ms | ~150–180 ms | ~66 ms |
| `parse_lemon.tcl` | 7.4k / 177 | ~161 ms | ~157 ms | ~223 ms | ~110 ms |

Component split of the per-edit CU build (`parse_lemon`, from the prior
proc-loop instrumentation, still representative): `lower_to_ir` **~26 ms** +
module `build_cfg` **~10 ms** + the per-procedure clone/rebase loop **~52 ms** +
`with_interprocedural` **~19 ms**.  The 52 ms is *not* lattice compute (memoised)
— it is the offset-invariant plumbing run for **every** procedure every edit:
clone the proc's offset-0 IR body + `rebase_script`, then deep-clone the
memoised `FunctionUnit` + `rebase_function_unit` to the proc's real span
(`lattice_rebase.rs`).  The rebase target (the offset) shifts whenever an edit
moves a procedure, so the rebased unit can't be cached across edits.

Two ways to kill it, both larger refactors.  Neither alone is a knockout — they
attack disjoint parts of the floor.

### Approach A — incremental per-item IR lowering / CFG (attacks the ~26+10 ms)

Lower **per item-body**, keyed on the offset-0 body text (mirroring
`function_lattice` for CFGs and the per-item analyser walk for diagnostics), so
a body edit re-lowers one body and reuses the rest.  `item_tree` already gives
the body spans; `build_cfg`'s per-proc loop already builds each CFG from one
body, so the CFG half is nearly free to make per-item.

- **Pro:** reuses the *proven* offset-0 + rebase machinery (byte-identical for
  lattices today); does **not** touch the ~50 span-emit sites in the consumers
  (see Approach B's surface).  Architecturally consistent with slices 2–4.
- **Con / blocker:** lowering is **not** body-local.  `lower_to_ir_with_config`
  runs whole-module passes whose output for one body depends on the others —
  `specialise_factories`, `inline_uplevel_passthrough`, `extract_oo_methods_pass`,
  and `populate_trace_facts` (scans every body for `trace add execution …`,
  setting module-wide `traced_commands` / `has_dynamic_trace` that GVN reads).
  So a per-body lowering memo needs the same "cross-item facts live in the
  aggregate, fed in as an input" split the analyser walk used (Phase B/C): the
  body memo keys on the module-wide trace/factory facts, which a signature-class
  edit invalidates.  Tractable but non-trivial; gated by the same corpus
  differential.
- **Ceiling:** ~36 ms (lower+cfg).  The 52 ms rebase/clone **remains** — A does
  not remove it.  `collect_call_site_constants` reads the module CFG, so the
  per-proc CFGs can't simply be skipped; rework it to walk IR call statements
  directly first (small, independent, unblocks skipping redundant CFG builds for
  memoised procs).

### Approach B — offset-aware diagnostic consumers (attacks the ~52 ms)

Leave each `function_lattice` unit at **offset 0** and hand every consumer the
proc's base byte-offset, added at span-emit time — eliminating both the
`rebase_function_unit` walk **and** the deep-clone (consumers borrow the cached
`Arc<FunctionUnit>` + offset).  This is the single biggest lever (~52 ms).

- **Surface (from the consumer span-read audit).**  Four consumers read
  *absolute* spans out of a unit's `cfg`/`ssa`/`sccp`:
  - **taint** (via `with_interprocedural`) — **2 sites** (`taint.rs:1680,2313`);
    trivially offset-at-emit.  *Best first target.*
  - **analyser CFG/SSA tail** (`emit_cfg_ssa_diagnostics`) — **~8 sites** across
    5 emitters (W220/W211/H300/W210/W307); moderate.
  - **`run_all_checks`** — **~15–20 sites** across `gvn.rs` / `shimmer/span.rs` /
    `taint.rs` / irules; one aggregator, many helpers.
  - **`optimise_unit`** — **~25+ sites** across **9 passes**; widest.
- **Leakage risks (a pure offset-0 unit + whole-file `source` don't mix).**
  - `optimiser/propagation.rs` validates `source[span.start()..span.end()]`
    against the body text (O102) — with an offset-0 span over a whole-file
    `source` this slices the wrong bytes.  Needs offset-aware slicing (add the
    base before indexing) — the principal correctness hazard.
  - the analyser's W307 / `$var` branches already read the **full** `self.source`
    (deferred to the tail in Phase C); those stay absolute and must not be
    offset-shifted.  So the analyser becomes a **hybrid** (offset-0 unit spans +
    absolute source scans) — workable but fiddly.
  - sort-by-`span.start()` determinism (gvn/shimmer/optimiser) is offset-shift-
    invariant, so *relative* ordering is safe — but a build that mixes offset-0
    and absolute spans in one sort would corrupt it.  All-or-nothing per unit.
- **All-consumers-or-no-win constraint.**  `file_analysis_incremental` and
  `compiler_check_diagnostics` **share one** built `compilation_unit`.  If even
  one consumer (the optimiser, the hard one) still needs absolute spans, the
  shared unit must still be rebased → **no win**.  So B only pays off once
  *every* consumer of the shared unit is offset-aware, or the shared-unit query
  is split so the offset-aware consumers take an un-rebased unit.  A phased
  "taint + analyser first" rollout validates the machinery but banks **zero**
  wall-clock until the optimiser is converted too.

### Recommendation

Sequence by risk-adjusted value:

1. **Micro-win — *not* as cheap as it looks.**  Reworking
   `collect_call_site_constants` to walk IR call statements (so memoised procs
   needn't have their real-offset CFG built in `build_cfg`) is **divergence-prone**
   and was deferred:
   - The module CFG omits calls inside **top-level deferred loops** when
     `defer_top_level = true` (the codegen path), while a naive recursive IR walk
     would include them.  A faithful IR walker must replicate that `defer_top_level`
     gating exactly.
   - `build_for_inner` (hence `collect_call_site_constants`) is shared by the
     **codegen** build too, and `param_constants` seed SCCP — so a *uniform*
     behaviour change isn't caught by the memoised-vs-uncached
     `compiler_check_corpus` differential (both sides change together); only the
     pinned analyser unit tests + e2e would catch it.  Do this only with those
     pins watched, and prove the `defer_top_level` parity first.
   - **Why it matters:** on a *warm* edit `function_lattice` is a cache hit, so the
     proc's offset-0 CFG is already built; `build_cfg`'s real-offset per-proc CFG
     is then **pure redundant work** (~10 ms on `parse_lemon`).  Skipping it needs
     call-site collection off the CFG first — hence this micro-win is the unblock.
2. **Approach A** (incremental per-item lowering): lower risk (no consumer
   surface), architecturally consistent, ~36 ms ceiling — but first prove the
   body-lowering memo is byte-identical under the whole-module trace/factory/
   uplevel passes (the cross-item-facts-as-input split).
3. **Approach B** last and only committed-to wholesale: stage the machinery on
   **taint** (2 sites) + the **analyser tail** (~8) behind a `SpanOffset` newtype
   to prove byte-identity on a narrow surface, then convert `run_all_checks` and
   finally the **optimiser** (resolving the `propagation.rs` source-slice
   leakage) — banking the ~52 ms only when the last consumer flips.  Highest
   value, highest risk; do not start partial expecting a wall-clock win.

> **Backlog #2 (memoise `optimise_unit` per-function) is the optimiser half of
> Approach B, not an independent item.**  `optimise_unit`'s passes read
> `ctx.source[span]` at real offsets (`propagation.rs` O102), so an
> *offset-invariant* per-function optimiser memo — the only kind that survives the
> shifts a real edit causes — needs exactly the offset-aware/source-slice refactor
> Approach B's optimiser step describes.  A non-offset-invariant memo (keyed on the
> proc's real-offset unit) would miss on every edit that shifts the proc, so it is
> not worth building.  **Do #2 as part of #3-B's optimiser conversion**, not before.

All steps keep the full-rebuild fallback and gate on
`compiler_check_memo_matches_uncached_over_corpus` + `per_item_corpus` +
`differential_incremental` + e2e (the existing byte-identity contract).

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

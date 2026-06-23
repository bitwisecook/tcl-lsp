# SRV-INCREMENTAL — making the per-edit pipeline incremental: measurement & track design

> **Status:** Design + measurement. This document scopes the **SRV-INCREMENTAL**
> track: finishing end-to-end per-edit incrementality so a keystroke recomputes
> only what the edit actually changed — *within a file and across the project*.
> It **supersedes the SRV-ROPE track** (a rope-backed `DocumentState`): a measured
> experiment showed a rope addresses ~**0.02%** of per-edit latency, so the rope
> survives here only as an optional, late micro-optimisation. The per-item
> analyser firewall this track builds on is designed and largely shipped in
> [`../rust/incremental-analysis.md`](../rust/incremental-analysis.md); this doc
> owns what remains, and adds the cross-file dimension that doc does not cover.
>
> Claims here are tagged **measured** (a harness in this repo backs them) or
> **hypothesis** (with the prerequisite experiment and dependencies that must
> precede the work) — see the verification-status table after the task list.

## TL;DR — the decision

The per-edit critical path is dominated by **whole-file `run_all_checks`**, not by
buffer edits, re-lex, or the per-item analyser walk. Measured on `linalg.tcl`
(2 299 lines), warm salsa db, one single-character edit inside a proc body:

| per-edit work | cost | incremental today? |
|---|--:|:--|
| buffer apply + `LineIndex` rebuild (the rope's slice) | ~85 µs (**0.02%**) | n/a — trivial |
| analyser walk (`file_analysis_incremental`, warm) | ~85 ms | ✅ per-item memoised |
| **compiler checks (`run_all_checks` + `optimise_unit`), whole unit** | **~405 ms** | ❌ **whole-file every edit** |
| **warm per-edit total** (the two queries; checks dominate) | **~411 ms** | |

(The stages share one built `CompilationUnit`, so the warm total is roughly the
cost of the checks query — not the sum of the rows; full `tail_profile` dump below.)

So:

1. **The cost is one whole-unit taint fixpoint — and incrementalising it is hard.**
   The shipped salsa firewall already makes the analyser *walk* and the per-proc
   *lattices* incremental, but `run_all_checks` re-runs over the whole unit every
   edit (~99% of warm per-edit latency, measured). Decomposing it (measured): the
   per-function checks are only ~16 ms; **~385 ms is `solve_interprocedural_taints`,
   a whole-unit cross-proc entry-taint fixpoint** that — verified against the code —
   is *not* the memoised `taint_cascade` and depends on a proc's *callers*, so it
   does not reduce to a per-proc memo. The real win (Task 2b) is genuine incremental
   dataflow, not the easy wrap the first draft assumed.
2. **Cross-file analysis is not on the incremental graph at all.** The
   `WorkspaceIndex` is a plain server struct, `resolve_proc_call` is per-file,
   arity never crosses files, and editing file A recomputes **nothing** in file B.
   The cross-file cascade has to be *designed* (as salsa edges), not merely tuned.
3. **The rope was the wrong lever.** It speeds up buffer apply (0.02% of per-edit
   time) and costs 1.4–1.9× memory on many small files. It is demoted to an
   optional final task, gated on the analysis floor actually being gone first.

The track below sequences the cheap wins first, makes the dominant slice
incremental, then extends incrementality across files, and only then revisits the
rope.

## What the server does per edit today (the baseline measured)

Per `textDocument/didChange` (`rust/tcl-lsp-server/src/lib.rs`, `did_change`):

1. `apply_content_change` per content change → splice the buffer; bump the doc
   revision. (~µs; the rope's slice.)
2. `db_set_source` → salsa `SourceFile::set_text` with the **whole-file `String`**
   (one flat input per file; no chunk/per-proc granularity), bumping the input
   revision and marking every dependent query dirty.
3. `workspace_index.remove_document(uri)` — drops file A's symbols from the
   cross-document aggregate. **This is the only cross-file action, and it triggers
   no re-analysis of any other file.**
4. debounced `schedule_diagnostics` → two salsa queries:
   `file_analysis_incremental(file, config)` (the per-item analyser walk) and
   `compiler_check_diagnostics(file)` (`run_all_checks` + `optimise_unit` over the
   built `CompilationUnit`). For every dialect but `tcl8.4`/`f5-irules` the two
   share one `compilation_unit` (a same-revision cache hit).

The server's own comment marks the key gap: *"The re-analysis below is still
whole-document; bounding it to `reparse_window` is a documented follow-up — the
primitives exist in `tcl-lexer`."*

## Where the per-edit time actually goes (measurement)

Three harnesses pin the numbers down. Two are reproducible, workspace-excluded
experiments in this directory; the third is a committed production example.

```
# (a) apply-side numerator — what a rope speeds up, in isolation:
cargo run --release --manifest-path docs/design/srv-incremental/experiment/Cargo.toml
# (b) per-edit denominator — what that apply is a fraction of (real analyser + salsa db):
cargo run --release --manifest-path docs/design/srv-incremental/experiment-pipeline/Cargo.toml
# (c) production per-edit profile, warm db, single-char edit (the real server shape):
FILE=tmp/tcllib-2.0/modules/math/linalg.tcl \
  cargo run --release -p tcl-lsp-db --example tail_profile
```

Harness (c) on `linalg.tcl` (2 299 lines, 81 functions), one space inserted in a
proc body, alternating warm→edited:

```
== full per-edit path (salsa, memoised) ==
  file_analysis_incremental (per edit)     85.0 ms   ← analyser walk, per-item memoised
  compiler_check_diagnostics (per edit)   444.5 ms   ← WARM, still ~whole-file
  BOTH queries per edit (production)      411.2 ms   ← real warm per-edit latency
== compiler-check tail breakdown (whole-file, no memo) ==
  CompilationUnit::build_for + interproc   59.0 ms
  run_all_checks                          405.1 ms   ← dominates
  optimise_unit                            15.4 ms
```

That the checks re-run **whole-file** is measured directly, not inferred — the
`tail_profile` "re-execution breadth" tier counts salsa `WillExecute` events per
query across one body edit (and the `check_diagnostics_rerun_whole_file_on_body_edit`
test pins it):

```
== salsa re-execution breadth (one body edit, warm db) ==
  per-proc memoised (rebuild only the edited procedure):
    item_body_analysis            cold 80   1-edit 2
    function_lattice              cold 80   1-edit 1
    taint_cascade                 cold 80   1-edit 3
  whole-file (re-run in full regardless of which procedure changed):
    compilation_unit              cold  1   1-edit 1
    compiler_check_diagnostics    cold  1   1-edit 1
```

Editing one procedure rebuilds **one** `function_lattice` of 80 — the per-proc
memo works — yet `compiler_check_diagnostics` re-executes and runs `run_all_checks`
over **all 81 functions**. `run_all_checks` is not itself a salsa query (it emits
no `WillExecute` event); it runs inside that single whole-file re-execution with no
per-proc reuse. The timing corroborates: warm `compiler_check_diagnostics`
(~445 ms) ≈ no-memo `run_all_checks` (~405 ms). On `practcl.tcl` (~8.5k lines) the
same warm per-edit is ~1.6 s, scaling with file size exactly as a whole-file pass
would.

A 5–12× speedup on the ~85 µs apply slice (harness (a), below) is invisible
against this. That is why the track targets the analysis floor, not the buffer.

## What is already incremental (the foundation)

The per-item analyser firewall is designed and largely shipped — full detail in
[`../rust/incremental-analysis.md`](../rust/incremental-analysis.md); the shape:

- **Per-body / per-proc memo, offset-invariant (the live firewall).**
  `item_body_analysis` (keyed on `ItemBodyKey`), `function_lattice` (keyed on
  `FnLatticeKey`: the offset-0 IR body + module context + params + dialect), and
  `taint_cascade` (keyed on `FnLatticeKey` + `TaintSummaryKey`) are all
  `#[salsa::tracked]`, demanded on the live path through `file_analysis_incremental`
  (per body) and `compilation_unit` (per proc). The keys are *offset-invariant*,
  so a proc merely **shifted** by edits above it is a cache hit. This is what
  bounds a body edit to the one-rebuild measured above; tests assert "exactly one
  body/lattice/cascade re-runs" on an unrelated edit, and "zero" on a blank-line
  prepend.
- **Signature-firewall queries (built and tested, not yet wired).** `item_tree`
  → `item_sigs` → `file_decls` are `#[salsa::tracked]` queries that extract proc
  headers and would let a body-only edit **backdate** the cross-item passes via
  early-cutoff — but they have **no production callers today**
  (`file_analysis_incremental` walks structure inline; the within-file signature
  firewall is not yet a salsa early-cutoff). They sit beside the file like
  `reparse_window`: ready substrate, unwired — and they are the natural basis for
  the project signature table in the cross-file design (Task 6).
- **Coverage.** The per-item fast path covers the large majority of the corpus
  (92.2% at the last `incremental-analysis.md` baseline); the rest falls back to a
  full walk on incomplete or `syntax_error` input.

Three principles this track inherits and must not break:

- **Correctness must rest on `incremental == fresh` differential fuzzing plus the
  full-rebuild fallback — never on the assumption that an edit is local** (item-
  locality is a *performance* heuristic only). **The gate is narrower than it sounds
  today:** the fuzzer that exists (`differential_incremental.rs`) compares only the
  analyser walk's `AnalysisResult.diagnostics`, and only on the *test-only*
  `analyse_incremental` path — **not** the compiler-check diagnostics, **not** the
  live salsa path, **not** multi-file. So for every new surface below the fuzzer is
  a *dependency to build*, not a safety net that already covers it (see the
  verification-status table after the task list).
- **Offset-0 + rebase-at-aggregation** is the established pattern (Approach B):
  each unit is built at offset 0 and consumers add `base_offset` at span-emit
  time. New per-item work follows this model.
- **Salsa input setters always bump the revision.** A per-item input must be set
  *only when the item-tree diff says it actually changed*, or every keystroke
  wakes all direct dependents (the E4/E8 finding). This rule is what makes the
  cross-file design below safe.

## Cascading changes — how incremental re-analysis stays correct and bounded

The heart of the track. "Recompute only what changed" is easy to assert and
subtle to get right, because one edit can legitimately invalidate work far away.
Two regimes: *intra-file* (mostly solved, one gap) and *inter-file* (greenfield).

### Intra-file cascade

Classify each edit by what the **structural diff** says changed:

**1. Body-only edit** — text changes inside one proc body; all signatures
unchanged.
- *What recomputes (correct, shipped):* the per-proc memo bounds it — one
  `item_body_analysis`, one `function_lattice`, and the handful of `taint_cascade`
  that transitively depend on the edited body re-run (measured above: 2 / 1 / 3 on
  linalg). Shifted siblings are cache hits via offset-invariant keys.
- *The gap:* the analyser walk's cross-item passes and `compiler_check_diagnostics`
  (`run_all_checks` + `optimise_unit`) re-run over the **whole** unit — the checks
  are not split per-proc, so all 81 functions are re-checked for a one-proc edit.
  → **Task 2.**

**2. Signature change** — a proc's params/arity/name/namespace change.
- The edited proc's **header** changes, so the cross-item interproc summary and the
  arity / W123 passes must recompute — callers of the changed proc must re-check
  arity (E002/E003). Sibling **bodies** are unchanged, so their per-proc memo holds.
- *How the cascade stays bounded (today vs. target):* today the whole file's
  cross-item passes re-run inline (the salsa signature-firewall queries are
  unwired). The refinement (Task 2/6) wires `file_decls` and models the
  call-site→callee-signature dependency as a salsa edge — an arity check keyed on
  the resolved callee's `signature` — so only **callers of the changed proc**
  re-check, not every proc in the file.

**3. Structural edit** — add/remove a proc, an unbalanced brace, anything that
changes the item set.
- The item set changes; the regions keyed on the changed span recompute. The
  structural-state index (`reparse_window`, `script_is_complete`, the
  bracket/brace/paren indexes already built in `tcl-lexer`) bounds re-lex /
  re-segment to the dirty span once wired in (**Task 5**). New/removed procs change
  the cross-item layer, cascading as in case 2.
- On incomplete or `syntax_error` input, fall back to a conservative full rebuild
  (already the pattern) — correctness over locality.

In every case the backstop is the single-file `incremental == fresh` fuzzer.

### Inter-file cascade — changing a proc signature in one file, affecting others

**Today this cascade does not exist** — confirmed in both the Rust and (legacy)
Python servers:

- The `WorkspaceIndex` (`tcl-lsp-core`) is a plain server-owned struct, **not a
  salsa input**. It aggregates proc/class *definitions* and call sites by
  qualified name for *editor features* (completion, cross-document go-to-def,
  references, rename, call-hierarchy).
- The analyser's `resolve_proc_call` resolves **only against the current file's**
  `all_procs`; cross-file arity (E002/E003) is never checked.
- On `didChange`, file A is removed from the index and only A is re-analysed.
  **File B is untouched** until B is next edited. The one cross-file signal — W123
  (unresolved command) suppression against the set of workspace proc names — is a
  one-directional *filter applied at B's next analysis*, not a reverse-dependency
  cascade.

So inter-file incrementality is greenfield: there is nothing to make *faster* —
there is a correctness/coverage feature (cross-file diagnostics) to build
*incremental from the start*, the right way, rather than bolting a hand-maintained
reverse-dependency map onto an off-graph index.

**The design — cross-file dependencies as salsa edges (the rust-analyzer model),
bounded by Tcl's dynamic dispatch.** This is a **sketch, not yet prototyped**; the
open risks and the spike that must precede committing it are listed after the steps:

1. **Lift the project signature table into salsa.** Add a project-level
   `project_signatures()` query (a def-index keyed by qualified name) that depends
   **only on each file's `item_sigs`/`file_decls`** — the signature-firewall queries
   that already exist (built and tested, just unwired; see above) — and never on
   bodies. Because it reads only firewall outputs, a body edit in *any*
   file leaves it unchanged → it backdates → **zero cross-file work**. Only a
   signature/decl change recomputes it; exposing each symbol as its own
   `signature(qname)` query (point 2) then gives **per-entry** early-cutoff, so a
   change to one symbol does not recompute consumers of the others (early-cutoff is
   per-query-output, so the per-symbol query — not the whole table — is what bounds
   the fan-out).

2. **Make cross-file resolution and arity tracked queries.**
   `resolve_cross_file(call_site) → Option<def_id>` reads `project_signatures`; the
   per-call arity check depends on `signature(def_id)`, which may live in another
   file. When file A's proc signature changes, salsa invalidates **exactly the call
   sites whose resolution points at that proc — in any file** — and recomputes only
   their arity check, not those files' whole analysis.

3. **Reverse-dependency invalidation falls out for free.** This is the entire
   reason to put the edges on the salsa graph: you never hand-maintain a reverse-dep
   map (the index has none today, which is *why* there is no cascade). Salsa already
   records "B's query read A's signature"; bumping A's signature input invalidates
   B's dependent query precisely, and nothing else.

4. **The E4/E8 input discipline scales to the project.** A keystroke in A bumps
   only A's input; but if A's *signature-table entry* is unchanged (a body edit),
   the project table backdates and no file B wakes. This is the firewall extended
   across files — and it is what stops every keystroke in a heavily-depended-on
   utility file from waking the whole workspace.

5. **Tcl's dynamic dispatch bounds precision — stated honestly.** Tcl resolves
   command names at runtime: `eval`, computed names, `uplevel`/`upvar`,
   `namespace import`, `interp alias`, `[$obj method]`. Cross-file resolution is
   therefore **best-effort and name-based** (the same heuristic the index already
   uses for go-to-definition). The cascade is *precise* for the statically
   resolvable subset (qualified or unambiguously-named procs); for the dynamic
   remainder it falls back to the conservative filter (a name that disappears from
   the project surfaces W123 on the dependent's next analysis, as today). The track
   promises **incremental, no-worse-than-today** cross-file diagnostics with precise
   invalidation for the resolvable subset — not sound cross-file arity where Tcl
   semantics forbid it.

6. **Fan-out is correctness, not a bug — but must be cheap per dependent.**
   Changing a widely-called utility's signature legitimately re-checks every caller
   across the project. The firewall keeps each dependent's recompute to its
   *arity/resolution* layer (cheap), not its whole analysis, and only when the
   *signature* (not the body) changed. Debounce + salsa cancellation (already in the
   server) absorb edit bursts; a keystroke that does not alter a signature wakes
   nobody.

7. **Correctness gate (must be built): a multi-file differential fuzzer.** Extend
   `incremental == fresh` to *project* scope — a corpus of edit sequences that
   include cross-file signature changes, proc add/remove, and `source` /
   `package require` graph edits — asserting the incrementally-maintained project
   diagnostics equal a from-scratch project rebuild. **No such fuzzer exists today**
   (the current one is single-file and analyser-only); building it is part of this
   task, not a backstop it can lean on.

**Open risks — the salsa-mechanics three are now retired by a spike; the
heuristic-edge one remains:**
- **Cycles — RETIRED (spike, measured).** Cross-file dependency graphs are
  routinely cyclic (mutual `source`, mutually-recursive cross-file calls); an
  unhandled salsa cycle panics. The `experiment-xfile/` spike models the
  cross-file resolved-signature fixpoint (`resolved(file) = max(own sig, imports'
  resolved)`) with salsa 0.27 fixpoint recovery (`cycle_fn`/`cycle_initial`,
  bottom = 0) and **converges an induced A↔B mutual-import cycle to the lattice
  join (5, 5) with no panic** — the policy is now *wired and proven*, not just
  available.
- **Reverse-dependency precision — VALIDATED (spike).** On a fan of N leaves
  importing one utility, changing the utility's **signature** recomputes exactly
  `U + N` dependents (101 / 1001 — measured); nothing else.
- **Scaling / early-cutoff — VALIDATED (spike).** A **body-only** edit to the
  utility (a field `resolved` does not read) wakes **zero** dependents — it is not
  even a dependency, so no leaf is dirtied. The per-signature recompute is
  O(dependents), not O(project); the project table is O(project) to *rebuild* but
  per-symbol early-cutoff bounds the fan-out (1000-leaf fan resolves in ~0.5 ms).
- **Heuristic edges vs. precise invalidation — STILL OPEN.** Tcl name resolution
  is best-effort, so the salsa dependency *edges* are heuristic — a wrong edge can
  leave a stale diagnostic that today's reanalyse-on-next-touch would not. The
  spike proves the *machinery* (cycles / invalidation / cutoff); it does **not**
  prove the edges are right. "No-worse-than-today" must still be *demonstrated* by
  the multi-file fuzzer, which does not exist.

*Prerequisite experiment — done:* `docs/design/srv-incremental/experiment-xfile/`
(run: `cargo run --release --manifest-path …/experiment-xfile/Cargo.toml`) settled
the cycle policy + measured reverse-dep/cutoff/scaling above. *Remaining
dependencies before Task 6 ships:* (a) lift a project-level file set into salsa
(`WorkspaceIndex` is a plain server struct today, **not** a salsa input — the XL
core of the task); (b) wire the signature-firewall queries
(`item_tree`/`item_sigs`/`file_decls`, built but unused) into a `project_signatures`
query over it; (c) build the multi-file `incremental == fresh` fuzzer to settle the
heuristic-edge risk.

## The work to do — SRV-INCREMENTAL tasks

Ordered so each ships independently green and the cheap, high-leverage wins land
first. A claim here is **measured** only where a harness in this repo backs it;
everything else is a **hypothesis**, tagged with the *prerequisite experiment* that
must validate it before the design is trusted and the *verification gate* (a
differential fuzzer) that must exist to ship it. Most of those gates do **not**
exist yet — the verification-status table follows the list.

1. **Persisted incremental `LineIndex` on the `String` store** *(S — **DONE**).*
   `DocumentState` now holds a `line_index` beside `text`; `LineIndex::apply_edit`
   patches it in place on edit (drop line-starts inside the splice; shift those
   after by the byte delta; insert one per `\n` in the new text) instead of
   rebuilding per change, and the doc-holding position-lookup sites reuse it
   instead of `LineIndex::new` per request. ~0 memory cost. *Gate (built):* patched
   index byte-identical to a rebuild over a 5000-edit fuzz corpus
   (`apply_edit_matches_rebuild_under_fuzz`) + the live-path index-consistency test.
   Measured value is small (the apply/lookup slice is ~0.02% of per-edit), as the
   TL;DR predicted — but self-contained and zero-regression.

2. **Per-procedure check memo + incremental interprocedural taint.** A
   `run_all_checks` cost-decomposition (profiler phase tier, `linalg.tcl`, 81
   functions) splits the ~405 ms sharply — **measured**, and it overturns this task's
   prior framing:

   | phase | cost | shape |
   |---|--:|---|
   | GVN (redundancy / partial / loop) | ~6 ms | per-function |
   | shimmer + thunking | ~10 ms | per-function |
   | **`solve_interprocedural_taints`** | **~385 ms** | **whole-unit fixpoint** |

   - **2a — per-function check memo** *(**DONE**; an earlier "blocked" finding was
     itself a bug).* SCCP / GVN / shimmer / thunking read only the `FunctionUnit`,
     so they are memoised per proc via `function_checks(FnLatticeKey)` on the
     offset-0 `function_lattice` unit (gathered by `proc_taint_solve` alongside 2b,
     fed to `compiler_check_diagnostics` through `push_taint_and_module_checks` +
     `sort_diagnostics`). **Correction of the record:** a first attempt was reverted
     as "coupled to the whole-module passes" — that diagnosis was **wrong**. The
     divergence was a **rebasing bug**: `function_lattice` returns the *offset-0*
     unit (`rebase_function_unit` rewrites spans to absolute only inside the
     whole-module build), so the memo's checks must be rebased by the proc's
     `body_offset` — I had used `shift`/`abs_span`, which is identity at
     `base_offset == 0`. The corpus then surfaced a second subtlety: an O100
     constant branch with a `None` span lowers to the `(0, 0)` "unknown" sentinel,
     which the whole-module build rebases as an `Option` (`None` stays `None`)
     *before* lowering — so `(0, 0)` must **not** get `body_offset` added. Both
     fixed; byte-identical over the full release corpus (893 files). **Measured:**
     `compiler_check_diagnostics` 107 → **83 ms**, per-edit 154 → **125 ms** — more
     than the ~16 ms ceiling estimate (the warm memo also skips unchanged procs'
     full per-function recompute).
   - **2b — incremental interprocedural taint** *(XL; the real ~385 ms, and harder
     than the prior draft claimed).* `solve_interprocedural_taints` is **not**
     equivalent to the memoised `taint_cascade` (verified by reading the code): it is
     a richer **whole-unit fixpoint** that flows tainted call arguments into callee
     parameters (cross-proc *entry-taint* — the case `cross_proc_entry_taint_into_sink_warns`,
     which `fu.taints` / `taint_cascade` miss). So a proc's solved taint depends on its
     **callers**, a *reverse* dependency `taint_cascade`'s callee-only `TaintSummaryKey`
     does not model — it cannot be served from `taint_cascade`. A whole-unit memo keyed
     on summaries is also unsound: the solve reads each proc's real CFG/SSA via
     `run_propagation`, which a body edit changes. Incrementalising it needs a bounded
     entry-taint worklist that re-propagates only the edited proc and its
     taint-reachable neighbourhood — real incremental dataflow with its own
     correctness model.
     *Experiment (done):* profiling inside `solve_interprocedural_taints` (env-gated,
     reverted) found the ~385 ms is **not** the entry-taint worklist (~5 ms) but the
     **summary fixpoint**: `infer_proc_summary` ran 240 times (3 full passes × 80
     procs), re-inferring *every* procedure on *every* pass though the summaries are
     monotone and mostly converge on pass 1.
     *Optimise step — landed:* a **dirty-set worklist** now re-infers a procedure only
     when one of its direct callees' summaries changed (callee→caller edges from the
     interprocedural call graph), with a debug-only full-pass guard asserting the
     result is a true fixpoint. Measured on `linalg.tcl`: `solve_interprocedural_taints`
     385 → 158 ms, `run_all_checks` 405 → 179 ms, warm per-edit 411 → 237 ms (~1.7×) —
     output verified identical to the round-robin over the whole tcllib corpus (the
     guard never fires; 2878 `tcl-compiler` + 59 taint tests green).
     *2b — incremental across edits — **SHIPPED**.* The worklist alone still re-inferred
     all procs on pass 1 of every solve (~120 ms floor). That is now memoised: a
     checks-path-only `proc_taint_solve(db, file, cfg)` query drives
     `converge_summaries_with` through `proc_summary_cascade(db, FnLatticeKey,
     SummaryDepsKey)` — the `taint_cascade` pattern applied to `infer_proc_summary`,
     keyed on the proc's offset-0 body + its callees' summaries — and feeds the result
     into `run_all_checks_with_solved`. An unchanged proc's summary inference is a cache
     hit, so a body edit re-infers only the edited proc + its caller cascade.
     **Measured** (`tail_profile`, `linalg.tcl`, warm db): `compiler_check_diagnostics`
     **230 → 107 ms (~2.1×)**, warm per-edit (both queries) **245 → 154 ms (~1.6×)**;
     the analyser walk is untouched (78 ms, no time-to-first-tokens risk). Combined with
     the worklist, the checks path is **107 ms vs the original 445 ms (~4.2×)**.
     **Design decision — the worklist drives the fixpoint, not salsa cycles.** Rather
     than make `proc_summary_cascade` a salsa fixpoint query (which would be the
     codebase's first `cycle_fn`/`cycle_initial` use and carry convergence-proof risk),
     the existing `converge_summaries` worklist keeps driving the iteration and mutual
     recursion; salsa only memoises the *per-proc* `infer_proc_summary`. The debug
     fixpoint guard re-runs the **real** `infer_proc_summary` against the final
     summaries, so it validates the memo's correctness as well as worklist convergence.
     **Resolved by experiment — the keys cannot be shared, they must be re-derived.**
     The first plan was to thread the per-proc `FnLatticeKey`s out of the shared
     `compilation_unit` as `BuiltUnit { cu, lattice_keys }`, to avoid a second lowering.
     That is **not possible**: a salsa `#[tracked]` return must be `'static`, and
     `FnLatticeKey<'db>` is `'db`-interned — threading it fails to compile (`lifetime
     may not live long enough`; also `CompilationUnit: Eq` is unsatisfied — `PartialEq`
     only). Confirmed against salsa 0.27 by a reverted spike. The finished
     `Arc<CompilationUnit>` carries only the *rebased* `FunctionUnit`s, not the offset-0
     bodies the keys are built from. **So `proc_taint_solve` re-derives them with its
     own `build_unit_with_keys`** (keys local, never threaded), returning a `'static`
     `InterprocTaintResult`. Two properties keep that duplicate build cheap and safe:
       - **Mostly cache hits, not a re-lower.** Its per-proc lattice/cascade demands
         route through the *same* `function_lattice` / `taint_cascade` memos the shared
         build populated, so it pays only the whole-file structural reassembly:
         **measured ~28 ms warm** vs ~57 ms cold — well under the ~120 ms floor removed.
       - **Off the time-to-first-tokens path.** `proc_taint_solve` is demanded only by
         `compiler_check_diagnostics` (debounced diagnostics), never by `semantic_tokens`
         / the analyser walk — the duplicate build cannot regress first-token latency.
     **Soundness subtlety found + fixed (the graphops case).** `infer_proc_summary`
     passes `Some(summaries)` to `propagate_taints` (the colour-aware return-summary
     transfer) — which `taint_cascade`'s path does **not** — so the reconstructed
     `summaries` map must have an entry for *every* proc, not just the reachable set:
     a resolved callee that is *absent* (rather than present-and-clean) makes
     `propagate_taints` fall through to its conservative bare-argument join and
     **over-taint** (`taint.rs`'s `summaries.get(&target)?`). `::struct::graph::op::distance`
     in `graphops.tcl` tripped the debug guard on exactly this; the fix seeds the whole
     resolution domain clean before overlaying the reachable real summaries.
     **Verified:** the `compiler_check` corpus differential (debug, guard live) is
     **byte-identical to the uncached solve over the whole tcllib + Tcl 8.4/8.5/8.6/9.0
     corpus** (~1500 files, 510 s, guard never fires); plus `taint_cascade_matches_uncached_under_edits`
     (cross-edit correctness through the new path), `proc_summary_cascade_reused_on_unrelated_edit`
     (breadth: cold 3 → unrelated edit 1), and a focused `graphops` regression.
     **Cost/benefit (measured, gated like the rope):** the duplicate-build delta
     (~28 ms) is well under the ~120 ms pass-1 floor it removes, so the
     "measure-then-decide" gate (the one the rope is held to) **passed** before this
     shipped — not an assumption. The win is on the *non-paramount* checks-diagnostics
     path; the analyser walk / first-token latency is untouched.
   *Verification gate — partially built:* the cold `compiler_check` corpus differential
   (memo vs uncached, debug guard live) now exists and passes over the whole corpus. A
   **random-edit** differential fuzzer (memoised `compiler_check_diagnostics` vs a fresh
   whole-unit `run_all_checks` under fuzzed edit sequences) is still to build — the
   cross-edit correctness is meanwhile pinned by `taint_cascade_matches_uncached_under_edits`
   (a hand-written callee-flip edit sequence) + the corpus differential + the debug
   fixpoint guard.

3. **Approach A — incremental per-item IR lowering / CFG** *(L).* Per
   [`../rust/incremental-analysis.md`](../rust/incremental-analysis.md): lower per
   item-body keyed on offset-0 body text. Blocked today because whole-module passes
   (`specialise_factories`, `inline_uplevel_passthrough`, `extract_oo_methods_pass`,
   `populate_trace_facts`) make one body's IR depend on the others; resolve with the
   same "cross-item facts as inputs" split the analyser walk used. Attacks the
   ~59 ms lowering floor.

4. **Approach B follow-ups** *(deferred — one half negligible, one coupled to
   Task 3).* Two sub-parts, both re-evaluated against measurement:
   - *Remove the per-proc deep-clone* (`cu.interproc.clone()` in `optimise_unit`):
     **measured 0.1 ms** on `linalg.tcl` (~0.09% of the post-2b checks path).
     Removing it means making `PassContext.interproc` a borrow/`Arc`, which churns
     **15+ call sites** (mostly tests passing an owned `InterproceduralAnalysis::
     default()`). 15-site churn for 0.1 ms is below the value bar — the rope
     tradeoff. Skipped.
   - *Per-function `optimise_unit` memo:* harder than 2a's per-function check memo,
     for two concrete reasons (not the 2a "coupling", which was a rebasing bug):
     (a) the optimiser reads **absolute `source[span]` slices** (e.g.
     `consumed_var_count` counts `$var` refs in the original source span), so an
     offset-0 memo would need the source window threaded + rebased, not just span
     arithmetic; (b) `optimise_unit` does **whole-module overlap selection**
     (`select_non_overlapping`) + group renumbering across *all* functions'
     optimisations, so the per-function raw optimisations would memoise but the
     selection/renumbering stays whole-module (the 2b shape). Tractable on the 2a/2b
     pattern, but a separate M-sized piece; not yet built.

5. **Wire the structural-state index into the live re-lex path** *(blocked — no
   consumer exists; coupled to Task 7).* The intent: bound `did_change`'s re-lex /
   re-segment to the dirty span via the already-built `reparse_window` /
   `script_is_complete` / bracket-brace-paren indexes. **Verified blocked:** there
   is **no windowed re-lex or incremental-segmentation API** to consume a dirty
   span — `Lexer` (`new` / `with_source_map`) always lexes the *whole* source, and
   `did_change` feeds the whole-file `String` to the salsa input, so the re-lex /
   structure extraction happens inside whole-file salsa queries (`item_tree`,
   `file_analysis_incremental`) keyed on the entire text. Wiring `reparse_window`
   today computes a dirty span **nothing consumes** (dead code). Bounding the
   re-lex requires either a chunk-addressable `SourceFile` input (so only the dirty
   chunk's tokens recompute) or an analyser that re-segments a window and merges —
   i.e. **Task 7's infrastructure**. So 5 is coupled to 7, not an independent
   *(M)*. (`script_is_complete` is already wired where it matters — incomplete-input
   fallback in `analyser/state.rs` / `per_item.rs` and the REPL; it is specifically
   `reparse_window`'s windowed re-lex that has no consumer.) *When unblocked, the
   gate:* dirty-span re-lex byte-identical to a full re-lex over the edit-fuzz corpus.

6. **Cross-file cascade** *(XL — salsa mechanics spiked, core implementation
   unbuilt).* Lift the project signature table into salsa (`project_signatures` over
   per-file `file_decls`), make cross-file resolution + arity tracked queries
   (reverse-dependency invalidation then falls out of salsa), and apply the E4/E8
   input-setting discipline project-wide. Brings cross-file arity / W123 onto the
   incremental graph with precise invalidation for the resolvable subset and a
   conservative fallback for dynamic dispatch. *Prerequisite experiment — **done**:*
   `experiment-xfile/` spiked the cross-file fixpoint and **retired the three
   salsa-mechanics risks** (cycle convergence, reverse-dep precision, body-edit
   early-cutoff — see the open-risks block, all measured). *Step 1 — **landed**:* a
   `Project` salsa input + `project_proc_names` query (the cross-file W123
   resolution domain) now aggregate per-file `file_decls` on the real graph, with
   the **cross-file firewall proven** (`project_proc_names_firewall`: a body edit in
   any file recomputes it zero times; a decl change, once). *Remaining (the XL
   core):* (a) the server must create/populate the `Project` input (today it owns a
   plain `WorkspaceIndex`); (b) cross-file resolution + arity tracked queries that
   *consume* `project_proc_names` / per-symbol signatures; (c) the multi-file
   `incremental == fresh` fuzzer to settle the **still-open** heuristic-edge
   correctness risk before any cross-file diagnostic ships. The graph foundation and
   its firewall are built; the resolution edges, the server wiring, and the
   soundness fuzzer remain.

7. **(Optional, late) rope-backed store + chunk-addressable salsa input** *(XL,
   gated).* The demoted SRV-ROPE work — full sub-task breakdown and measurements in
   the experiment below. Justified **only after** Tasks 2–6, and **only if** the
   apply-side 0.02% slice has grown into a non-trivial share once the analysis floor
   is gone *and* the many-small-docs memory regression (1.4–1.9×) can be held under
   ~1.2×. Sub-tasks: feature-flagged rope `DocumentState` with burst-coalescing;
   `LineIndex::from_rope_slice` + `Lexer::with_source_map` rope-slice re-lex in
   `tcl-lexer`; chunk-addressable `SourceFile` input so `set_text` interns only
   changed chunks; MVCC write-window minimisation. The experiment is the gate.

**Benches & gates** *(S, throughout).* Fold `tail_profile` into a committed
per-edit bench: assert **no time-to-first-tokens regression** (the paramount
metric is a full-buffer `didOpen`, which none of this touches) and track warm
per-edit latency on the corpus task-by-task as each lever lands.

**Ordering rationale / exit criteria.** Tasks 1–2 deliver the bulk of the realistic
per-edit win (cheap apply win + the dominant `run_all_checks` slice) with no rope
and no cross-file work. 3–4 close the lowering floor. 5 unlocks bounded re-lex (and
the rope). 6 is the cross-file feature, built incremental-first. 7 (the rope) lands
only if its slice has grown measurable and the memory regression is contained —
otherwise the `String` store is retained. The experiment is the gate, not an
assumption.

## Experiments & verification status

What is **measured** (a harness in this repo backs it) versus what is **hypothesis**
(needs the named experiment before the design is trusted):

| Claim | Status | Backing / experiment needed |
|---|---|---|
| Warm per-edit ~411 ms; `run_all_checks` ~405 ms (~99%) | **measured** | `tail_profile` timing tier (`linalg.tcl`) |
| One-proc edit rebuilds 1 `function_lattice`; all 81 functions re-checked | **measured** | `tail_profile` breadth tier + `check_diagnostics_rerun_whole_file_on_body_edit` test |
| Rope apply ≈ 0.02% of per-edit; 1.4–1.9× memory on many small files | **measured** | `experiment/` + `experiment-pipeline/` harnesses |
| `run_all_checks` ~405 ms = ~16 ms per-function + ~385 ms whole-unit `solve_interprocedural_taints` | **measured** | profiler phase-decomposition tier (`linalg.tcl`) |
| `solve_interprocedural_taints` is whole-unit; ~385 ms is the summary fixpoint (240 infers / 3 passes), not the ~5 ms entry-taint worklist | **measured** | env-gated solve profiling |
| Dirty-set worklist: `solve` 385→158 ms, per-edit 411→237 ms (~1.7×), output unchanged | **measured + verified** | re-profile + full-corpus debug fixpoint guard + 2878 tests |
| Worklist win holds on the merged tree (FE-OPT inliner): `run_all_checks` 405→177 ms, per-edit →231 ms | **measured** | re-profile post-merge + 59 taint tests (guard active) |
| Salsa cycle recovery available for 2b/Task 6 (mutual recursion / `source` cycles) | **verified** (dep) | `salsa/src/cycle.rs` + `benches/dataflow.rs` (`cycle_fn`/`cycle_initial`); on salsa 0.27 |
| 2b per-proc keys cannot be shared from `compilation_unit` (salsa return must be `'static`; cu carries only rebased `FunctionUnit`s) → dup-build required | **verified** (experiment) | reverted `BuiltUnit` threading spike — `lifetime may not live long enough` + `CompilationUnit: Eq` unsatisfied on salsa 0.27 |
| **2b memo shipped:** `proc_summary_cascade`+`proc_taint_solve` — `compiler_check_diagnostics` 230→107 ms (~2.1×), per-edit 245→154 ms (~1.6×); dup-build ~28 ms warm | **measured + verified** | `tail_profile` + full-corpus `compiler_check` differential (510 s, guard live, byte-identical) + cross-edit/breadth/graphops tests |
| **2a per-function check memo shipped** — `compiler_check_diagnostics` 107→83 ms, per-edit 154→125 ms | **measured + verified** | `function_checks(FnLatticeKey)` rebased by `body_offset`; byte-identical over the full release corpus (893 files) + graphops debug-guard regression. (A first attempt's "coupled to Task 3" reading was a rebasing bug, since corrected.) |
| Task 1: `LineIndex::apply_edit` patch == rebuild; persisted index wired into `DocumentState` | **measured + verified** (shipped) | 5000-edit fuzz gate + live-path index-consistency test; ~0.02% per-edit value as predicted |
| Task 5: `reparse_window` can be wired to bound re-lex today | **refuted** (code) | no windowed re-lex / incremental-segmentation consumer exists; `Lexer` lexes whole source, `did_change` feeds whole text to salsa — coupled to Task 7's chunk input |
| Signature-firewall + `reparse_window` substrate built but unwired | **measured** (code) | grep: no production callers |
| Task 2 "cuts ~405 ms to one-proc cost" (the easy framing) | **refuted** | decomposition: ~16 ms easy (2a) + ~385 ms hard whole-unit taint solve (2b) |
| Task 2 (2b) memo is sound | **measured + verified** (cold) | full-corpus `compiler_check` differential (memo vs uncached, debug guard live) passes; cross-edit pinned by `taint_cascade_matches_uncached_under_edits`. *Random-edit* fuzzer still to build |
| Task 6 cross-file salsa *mechanics* (cycle convergence, reverse-dep precision, body-edit cutoff) | **measured + verified** (spike) | `experiment-xfile/` — A↔B cycle → (5,5) no panic; sig change → exactly N+1 dependents; body change → 0 |
| Task 6 step 1: `project_proc_names` (cross-file resolution domain) firewalls on the real graph | **measured + verified** | `project_proc_names_firewall` — body edit re-runs it 0×, decl change 1× (in `tcl-lsp-db`, on `file_decls`) |
| Task 6 cross-file cascade is correct (heuristic edges) + scales as a real `project_signatures` | **hypothesis** | needs the workspace-input refactor + multi-file differential fuzzer — *neither exists* |

Differential-fuzzer coverage **today**:

| Surface | Edit-fuzzed `incremental == fresh`? |
|---|---|
| Analyser-walk diagnostics, test-only `analyse_incremental` path | ✅ `differential_incremental.rs` |
| Live salsa `file_analysis_incremental` | ⚠️ corpus equality only (no random-edit fuzz) |
| `compiler_check_diagnostics` (the checks) | ⚠️ cold corpus differential (memo vs uncached) ✅ + debug fixpoint guard; random-edit fuzz still to build |
| Cross-file / multi-file | ❌ none — **Task 6 must build it** |

## Experiment (evidence)

The two harnesses in this directory measured the SRV-ROPE decision and remain the
evidence for *why this track is incremental analysis, not a rope*. Harness (a)
depends on the **production** `tcl-lexer::LineIndex` and `ropey` 1.6; all inputs
are ASCII (byte == char == UTF-16 unit) so both arms do the same logical work,
isolating the structural difference. Numbers are indicative ratios from one dev-box
run, not absolutes.

### Edit application — ns per `didChange` carrying B edits (harness a)

Rope persists across edits; `flatten` is the `Rope::to_string()` the salsa input
forces; `rope_full = rope_edit + flatten`.

| size  | B  | string (ns) | rope_edit | flatten | rope_full | full ÷ string |
|------:|---:|------------:|----------:|--------:|----------:|--------------:|
| 1KiB  | 1  |         627 |       421 |     157 |       578 | 0.92× |
| 16KiB | 1  |       8 664 |       972 |     824 |     1 796 | 0.21× |
| 16KiB | 64 |     575 298 |    16 727 |     841 |    17 568 | 0.03× |
|256KiB | 1  |     274 556 |     1 225 |  10 097 |    11 322 | 0.04× |
| 1MiB  | 1  |   1 313 611 |     1 355 |  72 101 |    73 456 | 0.06× |
| 4MiB  | 1  |   7 375 485 |     1 527 | 353 275 |   354 802 | 0.05× |

The rope wins on apply ≥16KiB (the win is avoiding the `LineIndex` rebuild +
double-alloc, not the splice), and 20–500× on bursts (B=64; editors rarely send
burst `contentChanges`). But this is **apply machinery**, and apply is ~0.02% of
per-edit latency (above).

### High edit rate — 500 sequential single-edit `didChange`s (total ms, harness a)

| size  | string (ms) | rope_full (ms) | speedup |
|------:|------------:|---------------:|--------:|
| 16KiB |         4.5 |            0.9 |   5.1× |
|256KiB |        70.0 |            5.7 |  12.4× |
| 1MiB  |       301.6 |           37.1 |   8.1× |

5–12× on apply+flatten sustained — invisible end-to-end while `run_all_checks`
dominates per-edit latency.

### Memory — many small open documents (heap bytes, harness a)

| N    | file  | strings | ropes | rope ÷ string |
|-----:|------:|--------:|------:|--------------:|
| 1000 | 2KiB  |    1MiB |  2MiB | 1.43× |
| 5000 | 1KiB  |    4MiB |  9MiB | 1.90× |
|  200 | 16KiB |    3MiB |  3MiB | 1.02× |

The rope's B-tree leaf chunks cost **1.4–1.9× memory for small documents** — a real
downside for a workspace of many small iRules / config snippets, and one a `String`
store does not pay. This is the regression Task 7's gate must hold under ~1.2×.

### Why the rope cannot make salsa incremental

A rope **cannot** change the analysis floor. `set_text` interns a `String`, bumps
the input revision, and invalidates dependents regardless of how the buffer is
stored; the rope must *flatten* (O(n)) before every `set_text`. Real
incrementality requires the **input itself** to be chunk-addressable (Task 7) so
salsa interns unchanged chunks and the lexer re-lexes only the dirty span — and
even then it only attacks re-lex (tens of µs–ms), not `run_all_checks` (hundreds of
ms). The rope is the last and smallest lever; this track spends the first and
largest ones first.

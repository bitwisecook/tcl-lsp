# Incremental analysis — experiments, discoveries, and the reasoning behind the plan

This is the lab notebook for the per-item incremental-analysis effort: the
corpus we tested against, every experiment run (hypothesis → method → result →
discovery), how the methodology evolved, and how the discoveries produced each
decision in the plan. The plan and design live in
[`incremental-analysis.md`](incremental-analysis.md); the shipped perf work in
[`lsp-performance.md`](lsp-performance.md). **The experiment scripts are kept in
the tree** (see *Where the scripts live*) so every number here is reproducible.

## Corpus sources

The session-start hook fetches real Tcl source into `tmp/`. The experiments
sweep the `.tcl` files (the differential corpus tests additionally sweep the
`.test` suites):

| Source | `.tcl` files | lines | what it is |
|---|--:|--:|---|
| `tmp/tcl8.4.20` | 40 | 18,009 | Tcl 8.4 stdlib + perf scripts |
| `tmp/tcl8.5.19` | 46 | 27,613 | Tcl 8.5 stdlib |
| `tmp/tcl8.6.16` | 66 | 33,772 | Tcl 8.6 stdlib + `tests-perf/` |
| `tmp/tcl9.0.3` | 91 | 44,258 | Tcl 9.0 stdlib |
| `tmp/tcllib-2.0/modules` | 794 | 442,121 | tcllib (practcl, snit, math, struct, tepam, …) |
| **total (.tcl)** | **~1,037** | **~566k** | |

Single large files used for cost measurements: `practcl.tcl` (8,463 lines),
`tcl9.0.3/library/http/http.tcl`, `tcllib-2.0/modules/tepam/tepam.tcl`.

This is real, idiomatic, diverse Tcl — deeply nested namespaces, TclOO, procs
defining procs, dynamic command names — which is exactly why the experiments
surfaced the edge cases they did.

## How we got to "per-item" (the reasoning)

1. The Python-vs-Rust benchmark showed Rust wins everywhere *except* heavy-edit
   latency. Root-causing that (a trivial request stalled ~1080 ms after an edit)
   found the **message loop was blocked by a synchronous full diagnostic**. That
   was fixed directly (async + debounced diagnostics; see `lsp-performance.md`).
2. With the loop unblocked, the **diagnostic itself still took ~1.4 s** per edit
   on practcl. We measured the breakdown and found the **whole-file
   `Analyser::analyse` walk dominates (~82%)**, parsing is ~3 ms, the
   optimiser/checks ~18%. `analyse_incremental` (the existing incremental path)
   *re-walks every command*, so it does not help.
3. Conclusion: the only lever is making the **walk itself incremental** —
   recompute only the edited proc. That is per-item analysis, and it is a large,
   correctness-sensitive refactor of a crate the differential corpus pins
   exactly. So before building, we ran **falsifiable experiments** to prove the
   load-bearing assumptions, with a full-rebuild fallback + fuzzer as the
   correctness contract (item-locality is treated as a *perf* heuristic, never
   the correctness guarantee).

## The experiments

### E3 — cost split (is per-item worth it?)
*Hypothesis:* the per-proc body walk dominates; the cross-item aggregate
(W123/arity/interproc) is cheap. *Method:* `analyse_commands(finalise=false)`
(walk) vs `finalise=true` (walk+tail) on practcl. *Result:* walk **~82%**
(~785 ms), tail **~18%** (~167 ms). **GO** — per-item attacks the dominant cost;
the unavoidable per-edit aggregate floor is ~167 ms (itself incrementalisable
later).

### E1 — item-locality (the firewall)
*Hypothesis:* a proc body's facts depend only on its own text + cross-item
signatures, not on sibling bodies. *Method (final form):* insert a benign
comment at one proc's body start (always valid; shifts only following offsets),
then assert every item/diagnostic ending *before* the insert is byte-identical.
*Result:* **345/349 files (98.9%) clean**; the only leaks are global-variable
diagnostics (W210 read-before-set / W211 set-but-unused) that span procs.

*Methodology journey (why the first runs lied):* the first perturbation blanked
the body to spaces — which **clobbered the body's opening brace**, unbalancing
the enclosing `namespace eval` so following siblings flipped namespace
(`::ns::X` → `::X`): 295/295 false "violations". Preserving braces+newlines
dropped it to a brace-balance-filtered 3 files. The robust comment-insertion form
finally gave the clean 98.9% signal. *Discovery:* item-locality holds for ~99%
of edits; **global-variable usage is a genuine cross-item fact**.

### E2 — relative-offset rebasing
*Hypothesis:* the analyser is offset-shift-invariant, so per-item facts produced
at offset 0 can be rebased by the item's start. *Method:* prepend K blank lines,
un-shift every fact by K, compare to the original `AnalysisResult`. *Result:*
**598/598 invariant.** **GO** — the slice-2 relative-offset model is sound.

### E4/E8 — salsa early-cutoff + cascade breadth
*Hypothesis:* a body edit re-executes exactly one `item_analysis`; a signature
edit re-runs the aggregate but no unrelated bodies. *Method:* a minimal salsa
graph (`Item{body,sig}`, `item_analysis`, `file_diag`) with a `salsa_event`
`WillExecute` counter. *Result:* body edit → **1** item; sig edit → aggregate,
**0** bodies; a body change with unchanged *output* cuts off the aggregate. **GO.**
*Discovery:* salsa **input setters always bump the revision** (no value-equality
cutoff on inputs) — so the build must set an item's input **only when the
item-tree diff says it changed**, else every keystroke wakes direct dependents.

### E6 — lattice costs
*Method:* time `CompilationUnit` build, `with_interprocedural`, `run_all_checks`,
`optimise_with_dialect` on practcl. *Result:* CU ~118 ms / 115 procs ≈ **1 ms
per proc**; interproc fixpoint **~10.6 ms**. **GO** — per-proc lattice recompute
+ interproc cascade is cheap.

### E7 — shared CompilationUnit
*Result:* `optimise_with_dialect` rebuilds its own CU+interproc (~129 ms
redundant); sharing one unit recovers it. **GO — shipped** (`optimise_unit`).

### E5 — the differential fuzzer (`incremental == fresh`)
*The correctness backbone.* Random edit sequences; assert `analyse_incremental`
== `analyse` at every step. *Result:* **the fuzzer found a real gap** —
`analyse_incremental` diverges from `analyse`. Root-caused: even **unedited**,
`analyse(src) != analyse_commands(src, segment(src), true)` — **`all_procs`
match, but diagnostic sets differ**. *Discovery:* the structural analysis is
already incremental-consistent; the per-item build's actual job is making
**diagnostic emission byte-identical** to `analyse`. The fuzzer is the permanent
gate that will hold it there.

## Discoveries → plan decisions

| Discovery (experiment) | Decision in the plan |
|---|---|
| Walk is 82% of cost (E3); `analyse_incremental` re-walks everything | Make the *walk* per-item; that's the whole effort |
| Bodies are 99% item-local; global vars leak (E1) | Firewall = signatures (cross-item) vs bodies (item-local); **global-var flow → full-rebuild fallback** (not an aggregate) |
| Analyser is fully offset-invariant (E2) | Per-item facts produced at offset 0, **rebased** by the item span (rust-analyzer ItemTree model) |
| Early-cutoff works; inputs always bump (E4/E8) | Cascade by salsa demand; **set an item input only when the item-tree diff says it changed**; bail-out to full rebuild when the frontier is too wide |
| Lattices ~1 ms/proc, interproc ~10 ms (E6) | Per-`FunctionUnit` lattices + an interproc **worklist fixpoint with a change-digest** |
| Shared CU saves ~129 ms (E7) | **Shipped** — `optimise_unit` over one CU |
| Fuzzer found `analyse` ≠ `analyse_commands` on diagnostics (E5) | **Correctness rests on the fuzzer + full-rebuild fallback, not item-locality**; slice 2's job is diagnostic byte-equivalence |

The overarching reasoning the maintainer steered to: **incremental must always
converge to exactly the full-rebuild answer.** So correctness is guaranteed by
(1) a full-rebuild fallback whenever incremental can't prove equivalence and
(2) the differential fuzzer proving `incremental == fresh` under random edits —
*independent* of how clean the firewall turns out to be. The invalidation
frontier is a bidirectional worklist fixpoint (a re-analysed dependent can change
its own signature, re-triggering its dependents) bounded by the bail-out.

## Baseline results snapshot (for later comparison)

Captured to compare against after slices land (re-run the scripts in *Where the
scripts live* and diff). Numbers are wall-time and move with machine load —
treat the **ratios/percentages and pass-counts** as the stable signal.

```
environment: 2026-06-13  commit 1aff3670  rustc 1.96.0  4 cores

E3 (practcl, 8463 lines)        walk 801.7 ms | walk+tail 950.4 ms | tail 148.7 ms (16%)  GO
E1 item-locality                349 files: 345 clean, 4 leaky (98.9%)  [leaks = global-var W210/W211]
E2 offset-invariance            598 files: 598 invariant, 0 differ (100.0%)  GO
E6 lattice costs (115 procs)    CU 111.5 ms | +interproc 121.3 ms (interproc 9.9 ms)
                                +run_all_checks 148.8 ms | optimise_with_dialect 142.9 ms
E7 shared-CU saving             ~121 ms (redundant CU+interproc build)  GO
E4/E8 early-cutoff              1 passed (body→1 item; sig→aggregate, 0 bodies; output-unchanged cuts off)
E5 fuzzer                       found gap: analyse != analyse_commands on diagnostics (all_procs match)
```

Targets after the per-item slices land (the comparison we're aiming for):
- E3 stays the same (it characterises the *full* walk we're avoiding).
- **single-proc edit re-analysis: ~800 ms → low-ms** (slice 3; measure via the
  heavy-edit harness in `lsp-performance.md` and the E8 recomputation count).
- E5 fuzzer: **0 mismatches** once slice 2 makes diagnostic emission
  byte-identical (currently it documents the gap to close).

The Python-vs-Rust feature table (the other axis of comparison) is recorded in
[`lsp-performance.md`](lsp-performance.md).

## Where the scripts live (kept in the tree)

| Experiment(s) | Script | Run |
|---|---|---|
| E1, E2, E3, E6, E7 | `rust/tcl-compiler/examples/incr_experiments.rs` | `cargo run --release -p tcl-compiler --example incr_experiments` |
| E4, E8 | `rust/tcl-lsp-db/tests/early_cutoff.rs` | `cargo test -p tcl-lsp-db --test early_cutoff` |
| E5 (fuzzer) | `rust/tcl-compiler/tests/differential_incremental.rs` | `cargo test -p tcl-compiler --test differential_incremental -- --ignored` |

E1/E2 graduate into permanent regression tests as slices 1–2 land; E5 is the
permanent correctness gate for the per-item path; E3/E6/E7 are perf probes.

# Incremental analysis — the measurements behind the design

The evidence base for per-item incremental analysis: the corpus, each
experiment (hypothesis → method → result), and what each result establishes
about the design in [`incremental-analysis.md`](incremental-analysis.md). The
experiment harnesses are kept in the tree, so every number here is reproducible
— see *Where the harnesses live*.

## Corpus

The session-start hook fetches real Tcl source into `tmp/`. The experiments
sweep the `.tcl` files; the differential corpus tests additionally sweep the
`.test` suites.

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
defining procs, dynamic command names — which is why the experiments surface
edge cases a synthetic corpus would not.

Wall-clock numbers move with machine load; the **ratios, percentages, and
pass-counts** are the stable signal.

## The experiments

### E3 — cost split: is per-item worth it?

*Hypothesis:* the per-procedure body walk dominates, and the cross-item
aggregate (W123 / arity / interprocedural) is cheap. *Method:* time the walk
alone against walk-plus-tail on `practcl.tcl`. *Result:* walk **~82%**
(~785 ms), tail **~18%** (~167 ms).

**Establishes:** per-item recompute attacks the dominant cost, and the
unavoidable per-edit aggregate floor is the ~18% tail.

### E1 — item-locality (the firewall)

*Hypothesis:* a procedure body's facts depend only on its own text plus
cross-item signatures, not on sibling bodies. *Method:* insert a benign comment
at one body's start (always valid; shifts only following offsets), then assert
every item and diagnostic ending *before* the insert is byte-identical.
*Result:* **345/349 files (98.9%) clean**; the only leaks are global-variable
diagnostics (W210 read-before-set, W211 set-but-unused) that span procedures.

*Why the method matters:* the first perturbation blanked the body to spaces,
which clobbered the body's opening brace, unbalanced the enclosing `namespace
eval`, and flipped following siblings' namespace (`::ns::X` → `::X`) — 295/295
false violations. Preserving braces and newlines cut that to three files;
comment insertion gave the clean signal. A perturbation that changes the
document's structure measures the structure change, not locality.

**Establishes:** item-locality holds for ~99% of edits, and global-variable
usage is a genuine cross-item fact — handled by fallback rather than by
modelling an aggregate.

### E2 — offset invariance

*Hypothesis:* the analyser is offset-shift-invariant, so per-item facts produced
at offset 0 can be rebased by the item's start. *Method:* prepend K blank lines,
un-shift every fact by K, compare against the original result. *Result:*
**598/598 files invariant.**

**Establishes:** the offset-0-and-rebase model is exact, which is what makes the
per-body memo hit for a procedure that merely moved.

### E4/E8 — salsa early cutoff and cascade breadth

*Hypothesis:* a body edit re-executes exactly one item analysis; a signature
edit re-runs the aggregate but no unrelated bodies. *Method:* a minimal salsa
graph with a `salsa_event` `WillExecute` counter. *Result:* body edit → **1**
item; signature edit → the aggregate and **0** bodies; a body change whose
*output* is unchanged cuts off the aggregate too.

**Establishes:** the cascade behaves as designed — and that salsa **input
setters always bump the revision** (there is no value-equality cutoff on
inputs), so an item's body input must be set only when the item-tree diff says
it changed.

### E6 — lattice costs

*Method:* time the `CompilationUnit` build, the interprocedural fixpoint,
`run_all_checks`, and the optimiser on `practcl.tcl`. *Result:* unit build
~118 ms across 115 procedures (≈1 ms per procedure); the interprocedural
`pure`/effects fixpoint only ~10.6 ms.

**Establishes:** per-procedure lattice recompute plus an interprocedural cascade
is cheap enough to be worth memoising per item.

### E7 — sharing one CompilationUnit

*Method:* compare building one unit for both diagnostic consumers against each
building its own. *Result:* the second build costs ~129 ms of pure redundancy.

**Establishes:** the shared `compilation_unit` query, keyed on the lexer config.

### E5 — the differential fuzzer (`incremental == fresh`)

*The correctness backbone.* Random edit sequences, asserting the incremental
result equals a from-scratch result at every step. Its first run found a real
gap: even **unedited**, the pre-segmented entry point disagreed with `analyse` —
the procedure sets matched but the *diagnostic* sets did not.

**Establishes:** the structural analysis was already incremental-consistent, and
the hard part is byte-identical diagnostic *emission*. Correctness therefore
rests on the fuzzer plus the full-rebuild fallback, never on how clean the
firewall turns out to be — incremental must always converge to exactly the
full-rebuild answer.

## Where the harnesses live

| Experiment(s) | Harness | Run |
|---|---|---|
| E1, E2, E3, E6, E7 | `rust/tcl-compiler/examples/incr_experiments.rs` | `cargo run --release -p tcl-compiler --example incr_experiments` |
| E4, E8 | `rust/tcl-lsp-db/tests/early_cutoff.rs` | `cargo test -p tcl-lsp-db --test early_cutoff` |
| E5 | `rust/tcl-compiler/tests/differential_incremental.rs` | `cargo test -p tcl-compiler --test differential_incremental -- --ignored` |
| per-edit phase timing | `rust/tcl-lsp-db/examples/tail_profile.rs` | `cargo run --release -p tcl-lsp-db --example tail_profile FILE=…` |
| fallback distribution | `rust/tcl-compiler/examples/per_item_fallbacks.rs` | `cargo run --release -p tcl-compiler --example per_item_fallbacks` |

E5 is the permanent correctness gate for the per-item path; E3, E6, and E7 are
perf probes; E1 and E2 are pinned by the corpus differentials listed in
[`incremental-analysis.md`](incremental-analysis.md).

# object_lattice — object-handle lattice cost (issue #994 C5a gate M1): results

Measurement gate for stage **C5a** of the object-type-lattice unification
(issue #994, coupled with #1099). Design: `docs/design/compiler/object-type-lattice.md`.
Implementation: `rust/tcl-compiler/src/object_types.rs`. Harness:
`rust/tcl-compiler/examples/object_lattice_cost.rs`.

Reproduce:

```sh
# fetch the external corpus (git-ignored, pinned to the MANIFEST SHAs)
experiments/corpus/fetch_corpus.sh
CORPUS="experiments/corpus/georgtree experiments/corpus/adversarial \
        experiments/corpus/vendor experiments/corpus/repo-fixtures"
# per-file CU build / lattice / scope-keying cost, gate verdicts, edge mix
cargo run --release -p tcl-compiler --example object_lattice_cost -- $CORPUS
# behaviour equality of the empty-seed fast path (unit gate)
cargo test -p tcl-compiler --lib object_types
```

`REPEATS=n` (default 3) runs each timed phase `n` times and keeps the
minimum. Numbers below: `--release`, `REPEATS=3`, x86-64 Linux, one file at
a time (no parallelism), Rust 1.97.0.

## Hypothesis (restated)

C5a widens the object-handle carrier — the union map
`object_handle_classes` already produces, plus a **scope-keyed** twin, an
owner-span index, the collection map, the factory-return fact, and the
global-cell subset — and produces it once per analysis from the
`CompilationUnit` the CFG/SSA diagnostics already build.

The stop-gate question is **not** "is the lattice fast in absolute terms".
It is: **is the lattice cheap relative to the compilation unit it rides
on?** If it is not, C5b's consumers cannot afford to read it on the LSP
request path and the whole staged plan stops here.

## Metric definitions

- **CU build ms** — `CompilationUnit::build_for` + `with_interprocedural`,
  the pass the lattice rides on. Already paid by every analysis
  (`Analyser::analyse` → `emit_cfg_ssa_diagnostics`), memoised behind the
  salsa `compilation_unit` query.
- **lattice ms** — `object_handle_classes`: the harvest plus the VTA-lite
  fixpoint. This is what ships today.
- **+scope-keying ms** — `object_handle_facts` minus the above: owner
  attribution, the owner-span index, the collection map, and the two
  exported sub-facts. The *new* cost C5a adds.
- **rounds** — VTA fixpoint rounds actually run (0 when the empty-seed fast
  path fired, or when the callee-side early-out did).
- **bindings by edge kind** — the four VTA type-propagation edges
  (aliasing, proc return, proc parameter, constructor parameter).

## Corpus

154 Tcl files, 66,827 lines — the same corpus as `experiments/mro_eval`,
pinned by `experiments/corpus/MANIFEST.md`.

| source | files | lines |
|---|--:|--:|
| georgtree (SpiceGenTcl, tclopt, ruff, argparse, …) | 140 | 49,960 |
| tcllib (clay, oodialect, oometa, oooption, ooutil) | 6 | 3,360 |
| Tcl core `oo` tests (8.6 + 9.0) | 5 | 13,404 |
| repo OO fixtures | 2 | 32 |
| adversarial (hand-written) | 1 | 71 |

One reproduction caveat: `fetch_corpus.sh`'s shallow clone could not fetch
SpiceGenTcl's pinned `d9d7187` (GitHub refuses a shallow fetch of an
arbitrary SHA); the pin was recovered with `git fetch --unshallow` +
`git checkout d9d7187`. tcllib and the core `oo` tests were fetched
file-by-file from `raw.githubusercontent.com` (the session had no
`tmp/tcllib-2.0` / `tmp/tcl*` trees for the script's copy path); Tcl
8.6.16 has no `tests/ooUtil.test`, so the `oo`-test set is 8.6 `oo` +
`ooNext2` and 9.0 `oo` + `ooNext2` + `ooUtil` — 5 files, 13,404 lines,
matching the MANIFEST totals exactly.

## Headline result

```
measured 154 files, 66,827 lines
totals: CU build 1371.1 ms, lattice 5.0 ms (0.36 % of CU),
        +scope-keying 0.8 ms (0.06 % of CU)
```

| gate | threshold | measured | verdict |
|---|--:|--:|---|
| median lattice / CU build | ≤ 5 % | **0.24 %** | **PASS** (21× margin) |
| p99 lattice / CU build | ≤ 15 % | **1.43 %** | **PASS** (10× margin) |
| worst-file lattice time | ≤ 25 ms | **0.802 ms** | **PASS** (31× margin) |

**Worst file:** `experiments/corpus/georgtree/SpiceGenTcl/src/generalClasses.tcl`
— 3,038 lines, 0.802 ms of lattice on an 85.2 ms CU build (**0.94 %**), 2
fixpoint rounds, 3 handles. It is the worst file by *absolute* lattice time,
not by ratio; the worst *ratio* is 1.86 %
(`SpiceGenTcl/examples/ltspice/advanced/diode_extract.tcl`, 143 lines).

Scope-keying — the thing C5a actually adds, including the scoped source
resolution the `by_scope` fixpoint runs — costs **0.8 ms across the whole
corpus**, 0.06 % of the CU build and 16 % of the lattice itself. It is not a
measurable cost at LSP latencies.

Run-to-run spread across repeats of the whole sweep is ~10 % on the totals
(the harness shares the box with other builds; this sweep's CU-build total
is 13 % above the pre-scoped-propagation sweep's, which moves the lattice
column with it) and does not move any verdict: the tightest gate still
clears by 10×.

Scoped propagation — resolving each edge's source variable in its owning
scope for the `by_scope` fact — was added after review (see "Scoped
propagation" below) and cost nothing measurable: median ratio 0.22 % →
0.24 %, p99 1.46 % → 1.43 %, `+scope-keying` 0.8 ms → 0.8 ms, on a sweep
whose baseline CU build was itself 13 % slower.

## Per-file (slowest 15 by lattice ms)

| file | lines | CU build ms | lattice ms | lattice % of CU | +scope-keying ms | rounds | handles | scoped |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| `georgtree/SpiceGenTcl/src/generalClasses.tcl` | 3038 | 85.171 | 0.802 | 0.94 % | 0.264 | 2 | 3 | 3 |
| `georgtree/tclopt/tclopt.tcl` | 3843 | 137.230 | 0.645 | 0.47 % | 0.138 | 1 | 0 | 0 |
| `vendor/tcllib/clay.tcl` | 2227 | 92.637 | 0.251 | 0.27 % | 0.019 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/ltspice/specElementsClassesLtspice.tcl` | 1533 | 31.775 | 0.203 | 0.64 % | 0.023 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/ngspice/specElementsClassesNgspice.tcl` | 1491 | 30.497 | 0.195 | 0.64 % | 0.019 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/specElementsClassesCommon.tcl` | 1307 | 28.175 | 0.194 | 0.69 % | 0.027 | 1 | 0 | 0 |
| `georgtree/argparse/argparse.tcl` | 977 | 63.258 | 0.177 | 0.28 % | 0.000 | 1 | 0 | 0 |
| `georgtree/ruff/src/ruff.tcl` | 3060 | 78.705 | 0.147 | 0.19 % | 0.008 | 0 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/xyce/specElementsClassesXyce.tcl` | 1053 | 22.182 | 0.139 | 0.63 % | 0.007 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/ngspice/netlistParserClassNgspice.tcl` | 1123 | 41.077 | 0.135 | 0.33 % | 0.057 | 0 | 0 | 0 |
| `georgtree/SpiceGenTcl/examples/ltspice/advanced/diode_extract.tcl` | 143 | 5.715 | 0.106 | 1.86 % | 0.006 | 1 | 3 | 3 |
| `georgtree/SpiceGenTcl/examples/ngspice/advanced/inverter_optimization.tcl` | 190 | 7.971 | 0.103 | 1.30 % | 0.000 | 1 | 1 | 1 |
| `georgtree/SpiceGenTcl/examples/ngspice/advanced/verilog_a_magnetic.tcl` | 166 | 6.845 | 0.098 | 1.43 % | 0.010 | 1 | 13 | 13 |
| `georgtree/SpiceGenTcl/examples/ngspice/advanced/diode_extract.tcl` | 142 | 5.495 | 0.094 | 1.70 % | 0.005 | 1 | 3 | 3 |
| `georgtree/tcl_tools/gnuplotutil.tcl` | 861 | 34.207 | 0.091 | 0.27 % | 0.000 | 0 | 0 | 0 |

The `handles` / `scoped` columns are the sizes of `any_scope` and
`by_scope`. They are equal on every corpus file: no file in this corpus has
the same handle name bound in two different scopes, so scope-keying costs
nothing in *entries* here — it buys the ability to tell those apart when
they do occur, which is the rename-correctness case C5b needs.

## Empty-seed fast path

The C5a design flagged that the propagation's early-out checks the wrong
condition: it returns only when the **callee-side** maps (returns / proc
params / ctor params) are all empty, which is false for any file that
merely defines a proc. So a file with zero object seeds still paid a full
statement walk.

| files | lattice ms (fast path) | lattice ms (pre-#994 walk) | saved | saved / file |
|--:|--:|--:|--:|--:|
| 109 of 154 | 1.366 | 2.976 | 1.610 ms (**54.1 %**) | 0.0148 ms |

Biggest single-file win: `georgtree/ruff/src/ruff.tcl` (3,060 lines, no
object seeds) — **0.340 ms → 0.147 ms**, a 57 % cut on a file where the
entire propagation was provably dead.

**The obvious gate is unsound.** `if out.is_empty() { return; }` — the
literal check the design proposed — *loses real bindings*, because two
edges fire with an empty seed set:

| shape | binding lost by `out.is_empty()` |
|---|---|
| `proc make {} { return [Pin new] }` + `set c [make]` | `c → ::Pin` (proc-return edge; the object never lands in a `set` target the harvest sees) |
| `proc take {dev} {…}` + `take [listbox .l]` | `dev → listbox` (`arg_classes`' direct-constructor branch reads no seed) |
| `Wrap new [listbox .l]` | `inner → listbox` (same branch, constructor parameter) |
| `take [struct::graph]` | `dev → struct::graph` (naming-factory form) |

The shipped gate is therefore the three-part condition "no seeded handle
**and** no object-returning proc **and** no argument that could be a
bracketed registry constructor", the third part computed for free during
the harvest walk that is already running. All four shapes above are pinned
by `empty_seed_fast_path_is_behaviour_preserving`, which compares the fast
path against `object_handle_classes_full_walk` (the pre-#994 unconditional
walk, kept for exactly this purpose).

## Scoped propagation (post-review fix)

Owner attribution decides where a binding is *written*. Review (Codex on
PR #1127) found that is not enough on its own: the fixpoint was resolving
every edge's **source** against the scope-blind union, then attributing the
result to a specific owner. Given

```tcl
proc a {} { set x [Pin new] }
proc b {} { set x 0; set y $x }
```

`b`'s alias read `a`'s `x` and recorded `(::b, y) → ::Pin` — a false
**singleton** in the map C5b's rename edits and the "provably a different
class" refusal gate treat as authoritative. That is precisely the
wrong-rewrite hazard `by_scope` exists to prevent, so a scope-keyed *output*
over a scope-blind *propagation* is not a narrow map at all.

The fix resolves each edge's source in the scope that owns it:

| edge | source | resolved in |
|---|---|---|
| aliasing | the read variable | the **reading unit**, then its class for an instance variable, then nothing |
| proc return | the callee's return type | nowhere — not a variable read, so unit-independent |
| proc parameter | the call-site argument | the **caller**, bound in the callee |
| constructor parameter | the call-site argument | the **caller**, bound in the constructor |

Both facts advance in one walk, each reading back only its own map, so
`any_scope` is untouched — verified below, not assumed. Cost: nil (see the
headline table's last paragraph).

Guarded by `by_scope_does_not_import_another_units_binding_through_an_alias`
(the FP guard, Codex's exact shape) plus three TP twins:
`by_scope_alias_within_one_unit_still_propagates`,
`by_scope_instance_var_alias_reads_the_class_owner` (the #797 bridge under
scoped resolution), and `by_scope_cross_unit_call_edges_still_bind`.

## Bindings by VTA edge kind (corpus total)

| edge | bindings |
|---|--:|
| seed (harvest) | 86 |
| aliasing `set A $B` | 2 |
| proc return `set A [f]` | 2 |
| proc parameter `f $obj` | 0 |
| constructor parameter `C new $obj` | 0 |

(Edge counts are per-round *emissions*, not distinct bindings: an edge whose
source was already at its fixpoint is re-emitted and re-unioned as a no-op.
The proc-return count is 2 rather than 1 because the scope-keyed fact can
lag the union by a round, so one file runs one extra round.)

Fixpoint rounds: max **2**, median **0**; 122 of 154 files run no round at
all (109 via the new fast path, 13 more via the pre-existing callee-side
early-out).

This is the same picture `mro_eval` measured from the other direction: on
real TclOO corpora, objects are overwhelmingly *received*, not
*constructed*, in the scope that dispatches on them — the intraprocedural
seeds are few (86 across 66,827 lines) and the propagation edges fire
rarely. It also explains why the cost gates pass by more than an order of
magnitude: there is almost nothing to propagate.

Two consequences worth carrying into C5b:

1. **The lattice's value is not in its fixpoint.** Effectively every
   binding comes from the seed harvest, with three alias/return edges on top.
   C5b's
   ⊤-rate improvement must come from the *carrier* — one shared fact, read
   by all five dispatch consumers — not from the propagation getting
   cleverer.
2. **Cost is a non-issue at this scale, so the C5b gate is precision, not
   latency.** A wrong resolution is a rename that breaks code; the design
   already sets 100 % precision on hand-labelled newly-bound sites as the
   C5b bar, and nothing here argues for relaxing it.

## Differential gates (no behaviour change)

C5a adds a producer and no consumer, so the diagnostics must be
byte-identical. Four invariants, checked over **435 files** (the 154-file OO
corpus plus `samples/`, `editors/vscode/testFixture/`, and the compiler's
own fixtures):

| invariant | question | result |
|---|---|--:|
| `object_handle_classes` vs `object_handle_classes_full_walk` | does the empty-seed fast path — the only change to an existing output — move any consumer's map? | **0 divergences** |
| `object_handle_facts(..).any_scope` vs `object_handle_classes(..)` | does the scoped `by_scope` propagation perturb the union fact the shipping consumers read? | **0 divergences** |
| `analyse` vs `analyse_per_item` on `object_handle_facts` | does the new fact need a per-item merge? | **0 divergences** |
| `by_scope[(owner, name)] ⊆ any_scope[name]` | is the narrow map really a refinement of the wide one? | **0 violations** |

The first two are the load-bearing ones: between them they say the *only*
things an existing consumer can see — the union map — did not move, by
measurement rather than assertion. An earlier sweep of the first and third
over 1,388 files (Tcl 8.6.16 `library/` + tcllib 2.0 `modules/` + the OO
corpus) was likewise clean.

Also green:

- `cargo test -p tcl-compiler` — 7,733 tests, 0 failures (29 of them in
  `object_types`).
- `cargo test -p tcl-compiler --test per_item_corpus -- --ignored` — 818
  files, 25 whole-`AnalysisResult` mismatches, **all pre-existing**:
  `object_handle_facts` is identical between the two paths in every one.
- `cargo run -p tcl-lsp-db --example fa_diff` — `clay.tcl` 94=94,
  `generalClasses.tcl` 88=88, `ruff.tcl` 32=32 diagnostics, 0 either-only.
- `cargo run -p tcl-lsp-db --example cc_diff` — 0 memo-only / fresh-only on
  `clay.tcl` and `ruff.tcl`. `generalClasses.tcl` reports 20 fresh-only
  O100s; that is the pre-existing `function_lattice` memo divergence
  `cc_diff` was written to reproduce — A/B-checked identical with and
  without this change.

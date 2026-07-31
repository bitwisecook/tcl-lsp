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
totals: CU build 1213.2 ms, lattice 3.8 ms (0.32 % of CU),
        +scope-keying 0.8 ms (0.06 % of CU)
```

| gate | threshold | measured | verdict |
|---|--:|--:|---|
| median lattice / CU build | ≤ 5 % | **0.22 %** | **PASS** (23× margin) |
| p99 lattice / CU build | ≤ 15 % | **1.46 %** | **PASS** (10× margin) |
| worst-file lattice time | ≤ 25 ms | **0.648 ms** | **PASS** (39× margin) |

**Worst file:** `experiments/corpus/georgtree/SpiceGenTcl/src/generalClasses.tcl`
— 3,038 lines, 0.648 ms of lattice on a 79.8 ms CU build (**0.81 %**), 2
fixpoint rounds, 3 handles. It is the worst file by *absolute* lattice time,
not by ratio; the worst *ratio* is 1.55 %
(`SpiceGenTcl/examples/ltspice/advanced/diode_extract.tcl`, 143 lines).

Scope-keying — the thing C5a actually adds — costs **0.8 ms across the
whole corpus**, 0.06 % of the CU build and 21 % of the lattice itself. It
is not a measurable cost at LSP latencies.

Run-to-run spread across repeats of the whole sweep is ~5 % on the totals
and does not move any verdict: the tightest gate still clears by 10×.

## Per-file (slowest 15 by lattice ms)

| file | lines | CU build ms | lattice ms | lattice % of CU | +scope-keying ms | rounds | handles | scoped |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| `georgtree/SpiceGenTcl/src/generalClasses.tcl` | 3038 | 79.763 | 0.648 | 0.81 % | 0.172 | 2 | 3 | 3 |
| `georgtree/tclopt/tclopt.tcl` | 3843 | 121.984 | 0.475 | 0.39 % | 0.134 | 1 | 0 | 0 |
| `vendor/tcllib/clay.tcl` | 2227 | 77.332 | 0.182 | 0.23 % | 0.043 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/specElementsClassesCommon.tcl` | 1307 | 23.322 | 0.164 | 0.70 % | 0.039 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/ngspice/specElementsClassesNgspice.tcl` | 1491 | 24.736 | 0.162 | 0.66 % | 0.023 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/ltspice/specElementsClassesLtspice.tcl` | 1533 | 27.206 | 0.159 | 0.58 % | 0.037 | 1 | 0 | 0 |
| `georgtree/argparse/argparse.tcl` | 977 | 58.632 | 0.149 | 0.25 % | 0.000 | 1 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/xyce/specElementsClassesXyce.tcl` | 1053 | 18.619 | 0.116 | 0.62 % | 0.007 | 1 | 0 | 0 |
| `georgtree/ruff/src/ruff.tcl` | 3060 | 72.291 | 0.105 | 0.14 % | 0.024 | 0 | 0 | 0 |
| `georgtree/SpiceGenTcl/src/ngspice/netlistParserClassNgspice.tcl` | 1123 | 34.298 | 0.104 | 0.30 % | 0.054 | 0 | 0 | 0 |
| `georgtree/SpiceGenTcl/examples/ltspice/advanced/diode_extract.tcl` | 143 | 4.774 | 0.074 | 1.55 % | 0.003 | 1 | 3 | 3 |
| `georgtree/SpiceGenTcl/examples/ngspice/advanced/inverter_optimization.tcl` | 190 | 6.712 | 0.074 | 1.10 % | 0.000 | 1 | 1 | 1 |
| `georgtree/SpiceGenTcl/examples/ngspice/advanced/verilog_a_magnetic.tcl` | 166 | 5.654 | 0.073 | 1.30 % | 0.020 | 1 | 13 | 13 |
| `georgtree/tcl_tools/gnuplotutil.tcl` | 861 | 28.876 | 0.070 | 0.24 % | 0.002 | 0 | 0 | 0 |
| `georgtree/SpiceGenTcl/examples/ngspice/advanced/diode_extract.tcl` | 142 | 4.502 | 0.069 | 1.54 % | 0.002 | 1 | 3 | 3 |

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
| 109 of 154 | 1.012 | 2.323 | 1.311 ms (**56.4 %**) | 0.0120 ms |

Biggest single-file win: `georgtree/ruff/src/ruff.tcl` (3,060 lines, no
object seeds) — **0.248 ms → 0.105 ms**, a 58 % cut on a file where the
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

## Bindings by VTA edge kind (corpus total)

| edge | bindings |
|---|--:|
| seed (harvest) | 86 |
| aliasing `set A $B` | 2 |
| proc return `set A [f]` | 1 |
| proc parameter `f $obj` | 0 |
| constructor parameter `C new $obj` | 0 |

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

1. **The lattice's value is not in its fixpoint.** 89 of 89 propagated
   bindings come from the seed harvest and three alias/return edges. C5b's
   ⊤-rate improvement must come from the *carrier* — one shared fact, read
   by all five dispatch consumers — not from the propagation getting
   cleverer.
2. **Cost is a non-issue at this scale, so the C5b gate is precision, not
   latency.** A wrong resolution is a rename that breaks code; the design
   already sets 100 % precision on hand-labelled newly-bound sites as the
   C5b bar, and nothing here argues for relaxing it.

## Differential gates (no behaviour change)

C5a adds a producer and no consumer, so the diagnostics must be
byte-identical. Two corpus-wide differentials were run over **1,388 files**
(Tcl 8.6.16 `library/` + tcllib 2.0 `modules/` + this experiment's 154-file
OO corpus):

| differential | question | result |
|---|---|--:|
| `object_handle_classes` vs `object_handle_classes_full_walk` | does the empty-seed fast path — the only change to an existing output — move any consumer's map? | **0 / 1,388 divergences** |
| `analyse` vs `analyse_per_item` on `object_handle_facts` | does the new fact need a per-item merge? | **0 / 1,388 divergences** |

The first is the load-bearing one: the fast path is the *only* thing in
C5a that can change what an existing consumer (the optimiser, the
interprocedural seed, `type_infer`, semantic tokens) sees, so zero
divergence over 1,388 real files is what makes "no behaviour change"
a measurement rather than an assertion.

Also green:

- `cargo test -p tcl-compiler` — 7,726 tests, 0 failures (25 of them in
  `object_types`, including `any_scope_is_verbatim_object_handle_classes`
  and the four-shape fast-path equality gate).
- `cargo test -p tcl-compiler --test per_item_corpus -- --ignored` — 818
  files, 25 whole-`AnalysisResult` mismatches, **all pre-existing**: every
  one was checked and `object_handle_facts` is identical between the two
  paths in each (the 15 the describer could not pinpoint were probed
  field-by-field; the other 10 name pre-existing fields —
  `instance_classes`, `diagnostics`, `global_scope`).
- `cargo run -p tcl-lsp-db --example fa_diff` — `safe.tcl` 50=50,
  `init.tcl` 23=23, `clay/clay.tcl` 94=94 diagnostics, 0 either-only.
- `cargo run -p tcl-lsp-db --example cc_diff` — same three files, 0
  memo-only / fresh-only checks **and** optimisations.

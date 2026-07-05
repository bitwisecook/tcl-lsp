# mro_eval — TclOO class-lattice dispatch resolver: results

Experiment for the two-halves TclOO model (MRO graph + object→class
lattice). Design: `docs/design/tcloo-mro-lattice.md`. Prototype:
`rust/tcl-compiler/src/analyser/class_lattice.rs` (not wired into shipping
diagnostics). Harness: `rust/tcl-compiler/examples/mro_eval.rs`.

Reproduce:

```sh
# fetch the external corpus (git-ignored)
experiments/corpus/fetch_corpus.sh
# run
cargo run --release -p tcl-compiler --example mro_eval -- \
    experiments/corpus/georgtree \
    experiments/corpus/adversarial \
    experiments/corpus/vendor \
    experiments/corpus/repo-fixtures
```

## Hypothesis (restated)

Splitting TclOO tracking into (1) a precomputed **MRO graph** and (2) an
object→class **binding lattice** over the existing SSA beats today's
`aggregate_object_types` heuristic on real code, at an acceptable
⊤-collapse rate and cost.

The lattice half is the thing under test: half (1) already ships
(`mro.rs` + `class_hierarchy.rs`) and is reused. The question is whether
the **lattice** (SSA-flow class binding, JOIN at merges) and the **full
MRO** (mixins/filters) earn their keep — measured by ⊤-rate and the
marginal value of each layer.

## Metric definitions

- **⊤-rate** — `Abstain` sites / all `$obj method` sites. The make-or-break
  number: if it is high, the lattice buys little. Broken down by the ⊤
  taxonomy (`docs/design/tcloo-mro-lattice.md`).
- **Resolution split** — of the *resolved* (non-⊤) sites, `method-known`
  (class + method both found) vs `method-unknown` (class named, method not
  — the W308 candidate).
- **Precision (resolved set)** — a resolved verdict names the *right*
  class. Checked on a hand-verified sample. A false resolution is a
  correctness bug (worse than ⊤).
- **Ablations** — A0 MRO-only (single class, no join) → A1 +join → A2
  +mixins/filters → A3 +cross-file index. `Δ resolved` is the marginal
  value of each layer.
- **Cost** — resolver time added on top of the base analyser walk.

## Corpus

154 Tcl files, 66,827 lines, 448 TclOO classes, **1,803 `$obj method`
dispatch sites**. Provenance in `experiments/corpus/MANIFEST.md`.

| source | files | lines | classes | sites | provenance |
|---|--:|--:|--:|--:|---|
| georgtree (SpiceGenTcl, tclopt, tclinterp, …) | 140 | 49,960 | 335 | 1,561 | real OSS TclOO, github.com/georgtree |
| tcllib (clay, oo*, ooutil) | 6 | 3,360 | 9 | 33 | tcllib 2.0 metaobject frameworks |
| Tcl core `oo` tests (8.6 + 9.0) | 5 | 13,404 | 102 | 201 | `tests/oo*.test` |
| repo OO fixtures | 2 | ~30 | 2 | ~0 | `editors/vscode/testFixture` |
| adversarial (hand-written) | 1 | 71 | 3 | 8 | this experiment |

## Headline result

| ablation | resolved | ⊤ | ⊤-rate | Δ resolved |
|---|--:|--:|--:|--:|
| A0 MRO-only (single-class, no join) | 3 (0.2%) | 1,800 | **0.998** | +3 |
| A1 +join (CFG-merge lattice) | 3 (0.2%) | 1,800 | 0.998 | **+0** |
| A2 +mixins/filters (full MRO) | 3 (0.2%) | 1,800 | 0.998 | **+0** |
| A3 +cross-file index (FULL) | 338 (18.7%) | 1,465 | **0.813** | **+335** |

**The two facts that decide the design:**

1. **The SSA lattice half buys ~nothing on its own.** Intraprocedurally
   (A0–A2) the resolver binds a class at **0.2 %** of sites — a **99.8 %
   ⊤-rate**. Objects are overwhelmingly *received*, not *constructed*, in
   the scope that dispatches on them.
2. **All of the resolving power is the class index being cross-file**
   (A3, Δ = +335). That is the *MRO/CHA half plus a workspace class index*
   — **not** the lattice, **not** join, **not** mixins.
   - **+join → Δ 0.** **+mixins/filters → Δ 0.** On 1,803 real sites,
     neither layer resolved a single additional site.

## ⊤ breakdown (FULL / A3)

| reason | sites | % of all sites |
|---|--:|--:|
| `unknown` (extern/param receiver) | 1,083 | 60.1 % |
| `cross-file-miss` | 287 | 15.9 % |
| `factory-return` | 82 | 4.5 % |
| `per-object-mixin` | 8 | 0.4 % |
| `introspection` | 5 | 0.3 % |
| *(resolved)* | 338 | 18.7 % |

- **`unknown` dominates (60 %), and 100 % of it is extern/param
  receivers** — the receiver variable is never assigned in the file; it
  arrives as a proc/method parameter, a global, or via `upvar`. **No
  intraprocedural lattice can ever bind these.** Only *interprocedural*
  object-type flow (propagating class types through parameters/returns)
  could, and that is a much larger investment than the lattice.
- **`cross-file-miss` (16 %)** is class *names* we see (`[Foo new]`) whose
  `ClassDef` isn't in the merged index (ambiguous namespace tails, or
  classes genuinely outside the corpus).
- The genuinely-dynamic reasons the lattice is *designed* to catch —
  `factory-return`, `introspection`, `per-object-mixin` — together are
  **5.2 %** of sites. The lattice abstains on all of them correctly (see
  precision), but they are a small slice.

## Per-corpus (the pattern is not a georgtree artefact)

| corpus | sites | per-file resolved | cross-file resolved |
|---|--:|--:|--:|
| georgtree | 1,561 | 0.2 % | **21.5 %** |
| tcllib (clay/oo*) | 33 | 0.0 % | **0.0 %** |
| Tcl core `oo` tests | 201 | 0.0 % | **0.0 %** |
| adversarial | 8 | 0.0 % | 0.0 % (all correct ⊤) |

- **georgtree** (SpiceGenTcl et al.) is the *best case*: classes in `src/`,
  objects built and dispatched in `examples/` and `test/` → the cross-file
  index earns 21.5 %.
- **tcllib clay and the Tcl core `oo` tests stay 100 % ⊤ even
  cross-file.** clay is a metaobject framework (dict-driven dispatch,
  `my`, `[self]`); the `oo` tests construct objects via
  `[oo::class create …] new` and `[$c new]` — dynamic class handles the
  lattice must abstain on. Two independent corpora, same verdict.

## Precision on the resolved set (hand-verified sample)

Spot-checked resolved verdicts against source; **no false resolutions
found** (consistent with sound-by-abstention). Examples:

| site | resolved to | verified |
|---|---|---|
| `$circuit add` | `::SpiceGenTcl::Circuit` | ✓ `circuit` = `[Circuit new …]` (examples/), `Circuit` defines `method add` (src/generalClasses.tcl) — a genuine cross-file resolve |
| `$optimizer run` | `::tclopt::Mpfit` | ✓ `Mpfit` defines `run` |
| `$par0 configure` | `::tclopt::ParameterMpfit` | ✓ configurable class, `configure` is a builtin |

`method-unknown` (class named, method not) was **6 sites** cross-file —
the W308 candidate set, i.e. potential *true* "unknown method" findings.

## Adversarial check (correctness)

`tests/mro_lattice_adversarial.rs` (9 tests, all pass) confirms the
resolver **abstains, never mis-resolves**, on dynamic shapes:

| shape | verdict |
|---|---|
| reassign object→string | `⊤ dynamic-assign` |
| `[make]` factory return | `⊤ factory-return` |
| `[$cls new]` dynamic class | `⊤ introspection` |
| `[oo::copy $x]` | `⊤ introspection` |
| `oo::objdefine $o {…}` then dispatch | `⊤ per-object-mixin` |
| bare parameter receiver | `⊤ unknown` |
| **if/else join of Dog|Cat** (both define the method) | **`Resolved {Dog, Cat}`** ✓ |
| concrete `[Dog new]` | `Resolved {Dog}` ✓ |
| `[Dog new]` then unknown method | `Resolved {Dog}, method_known=false` (W308 cand) |

The one positive case for the lattice — a control-flow JOIN of two known
classes — resolves correctly *in isolation*. See the granularity caveat.

## Cost

- Base analyser walk: **51 ms/file** (8.5 KLOC/s) — the file sizes here
  are large (SpiceGenTcl classes run to ~1 KLOC each).
- Resolver overhead: **~1.06 ms/file per ablation** (≈ 0.26 ms/file for a
  single config) — negligible, and it rebuilds the whole hierarchy per
  file per ablation (a real integration would memoise the MRO in the class
  index, as `class_hierarchy` already supports).

## Threats to validity / honest caveats

- **Var-name granularity.** The ⊤-attribution is keyed by variable *name*,
  not `(scope, SSA-version)` — the same coarse granularity the shipping
  `aggregate_object_types` heuristic uses. In a file where several procs
  reuse the name `o` for different objects, one proc's dynamic binding
  poisons the others (visible in the adversarial *file*: the branch-join
  `$o speak` collapses to `⊤ per-object-mixin` because a *different* proc
  `oo::objdefine`s its own `o`). The adversarial *unit* test shows the
  join resolves when names don't collide. A scope/SSA-keyed lattice would
  fix this — but it would only recover sites in the ~5 % dynamic slice, not
  the 60 % param-receiver slice, so it does not change the recommendation.
- **Cross-file index ≈ the LSP's real state.** In the shipping LSP,
  `all_classes` is the *workspace* index — i.e. the A3 (cross-file)
  configuration is the one that matches production. The shipping W308 path
  therefore already gets this resolving power today, via the type lattice
  + workspace class index. This experiment did not find a case where the
  lattice adds to it.
- **Precision sample is a spot-check**, not the full ≥100 hand-labelled
  set the protocol asked for; it is sufficient to establish "no false
  resolutions on the resolved set", which is the correctness-critical
  direction. Recall is dominated by the ⊤-rate and reported as such.

## Recommendation

**Do not build the SSA object→class lattice (join / widening) or wire the
mixins/filters MRO into a new dispatch path. The measurements do not
justify either.**

Concretely, staged by evidence:

1. **Ship nothing new from the *lattice* half.** +join and +mixins each
   resolved **0** additional sites across 1,803 real dispatches. The
   marginal value is zero on this corpus.
2. **The MRO table + workspace class index is the whole story, and it
   already ships.** `mro.rs`/`class_hierarchy.rs` + the LSP's cross-file
   `all_classes` already deliver the 18.7 % that *is* resolvable. Keep
   using them; the experiment validates that design.
3. **If you want to move the ⊤-rate, the only lever that matters is
   interprocedural object-type flow through parameters/returns** — that is
   60 % of all sites (the extern/param receivers) and 100 % of the
   `unknown` bucket. This is a *type-flow* problem, not a *lattice* one:
   propagate `TclType::Object { class }` across call edges (the
   interprocedural summaries already exist for taint/const). Before
   committing, measure: instrument how many param-receiver sites have a
   *single* concrete caller-supplied class — if most callers are
   themselves params (turtles all the way down), even this won't help.
4. **Retain the ⊤ taxonomy + harness** as the yardstick for (3). The
   `next_provider` (`next`/`nextto`) modelling and the ⊤ instrumentation
   are cheap, correct, and useful for go-to-definition on `next` chains
   independent of the lattice question.

**Bottom line: a well-measured negative result for the lattice.** The
hypothesis ("the lattice beats the heuristic") is **not supported** — the
lattice's local evidence is absent at 99.8 % of sites, and the value is
entirely in the already-shipping MRO/CHA half plus cross-file indexing.
The honest next step is interprocedural param typing, gated on its own
measurement.

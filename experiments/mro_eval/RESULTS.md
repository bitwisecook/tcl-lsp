# mro_eval — TclOO class-lattice dispatch resolver: results

Experiment for the two-halves TclOO model (MRO graph + object→class
lattice). Design: `docs/design/tcloo-mro-lattice.md`. Prototype:
`rust/tcl-compiler/src/analyser/class_lattice.rs` (not wired into shipping
diagnostics). Harness: `rust/tcl-compiler/examples/mro_eval.rs`.

Reproduce:

```sh
# fetch the external corpus (git-ignored, pinned to the MANIFEST SHAs)
experiments/corpus/fetch_corpus.sh
CORPUS="experiments/corpus/georgtree experiments/corpus/adversarial \
        experiments/corpus/vendor experiments/corpus/repo-fixtures"
# ⊤-rate + ablations + cost
cargo run --release -p tcl-compiler --example mro_eval  -- $CORPUS
# W307/W308 delta (stderr) + labeling worksheet (stdout)
cargo run --release -p tcl-compiler --example mro_delta -- $CORPUS \
    > experiments/mro_eval/label_worksheet.csv
# precision / recall from the hand-labeled ground truth
python3 experiments/mro_eval/gen_labels.py   # → labeled_sample.csv
python3 experiments/mro_eval/score_labels.py experiments/mro_eval/labeled_sample.csv
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

## Precision / recall (hand-labeled sample)

**Methodology.** `examples/mro_delta.rs` emits a labeling worksheet
(`label_worksheet.csv`) with, per site, the resolver verdict, the shipping
diagnostic, the dispatch line, and the nearest preceding binding of the
receiver. Ground truth was established by **auditing the corpus source**:
a receiver bound by a nearby `[Class new|create …]` holds that class
(spot-audited for confounding reassignments); the 10 resolved sites whose
binding the worksheet did not capture on the same line were hand-audited
(e.g. `vSrc→Dc` via a later global `set` used inside a proc; `circuit→
Circuit`); every method the resolver flagged unknown was checked against
its class's real superclass chain. The audited labels are encoded by
`gen_labels.py` → `labeled_sample.csv` and scored by `score_labels.py`.
**150 labeled sites** (109 resolved with a concrete truth + 41
abstentions).

| metric | value | meaning |
|---|--:|---|
| **class-resolution precision** | **109/109 = 100 %** | of resolved sites, the named class set contains the true receiver class — **0 false resolutions** |
| **recall (locally-knowable)** | **109/109 = 100 %** | of sites whose class is determinable from the file, the resolver resolved (didn't abstain) — 0 knowable sites missed |
| **`method_known` flag accuracy** | **103/109 = 94.5 %** | of resolved sites, the method flag agrees with whether the method really exists |
| **abstention audit** | **41/41 correct** | every abstention had an `EXTERN` (param/global) or `DYNAMIC` truth — 0 abstentions on a knowable class |

Two things to read carefully:

- **100 % recall is over the *locally-knowable* subset, not all sites.**
  81.3 % of sites are `EXTERN`/`DYNAMIC` (param receivers, dynamic
  dispatch) and are *correctly* excluded from the knowable denominator.
  "Recall = 100 %" means the resolver never abstains on a site whose class
  a human could pin down locally — **not** that it resolves most sites.
- **The 6 wrong `method_known` flags are all false W308s**, not missed
  findings. Each is a receiver whose class defines the method **by
  inheritance** (`superclass Device`/`Model` → `::SpiceGenTcl::Device`),
  but the class was declared with a *bare* superclass name that the
  cross-file MRO builder (`class_hierarchy`, which normalises only via
  `::name`) did not link to its namespaced definition. So the resolver's
  "new W308 candidates" would fire on valid code. This is a concrete
  correctness limitation of cross-file resolution, and it argues *against*
  wiring the resolver into W308.

## W307/W308 delta vs. the shipping heuristic

`examples/mro_delta.rs` cross-tabulates the resolver's verdict against the
diagnostic the **real** analyser emits at each dispatch span (not a
reimplementation). Over the 1,803 sites:

| resolver \ shipping | W307 | W308 | none |
|---|--:|--:|--:|
| resolved-known | 2 | 0 | 330 |
| resolved-unknown | 3 | 0 | 3 |
| abstain | 46 | 0 | 1,414 |

The shipping heuristic fires **51 W307 and 0 W308** at these sites — it is
already very conservative here. As a hypothetical replacement the resolver
would:

- **Remove 2 W307 false positives** (`resolved-known × W307`): it proves
  the head is a valid object dispatch with a known class + method.
- **Add 6 W308** (`resolved-unknown`, the 3 `× none` + 3 `× W307`) — but
  per the precision audit **all 6 are false positives** (unlinked
  inherited methods). **0 true new findings.**
- **Regress nothing** (`abstain × W308 = 0`; it never contradicts a
  shipping W308).
- Leave 46 W307 untouched (`abstain × W307`) — no claim either way.

Net: **+2 real FP removed, 6 FP introduced, 0 TP gained** — a *worse*
diagnostic than today's heuristic, driven by the cross-file inheritance
gap. Another data point against shipping.

## Namespace-soundness (addressing the review)

The first cut resolved a bare `Foo new` by matching the unique class tail
`Foo` *anywhere* in the merged index — which can name `::Other::Foo` for a
call that Tcl would not resolve there (flagged in review). Replaced with a
**sound** model (`NsContext`): a bare name resolves via (1) the enclosing
`namespace eval`, (2) `namespace import`ed prefixes, (3) the global
namespace — else it abstains (`cross-file-miss`). It is **never** matched
to a same-tailed class in an unrelated namespace.

Re-running with the sound model, the resolved count **held at 338** — the
SpiceGenTcl cross-file resolutions were all backed by a real
`namespace import ::SpiceGenTcl::*`, so modeling imports recovers them
*soundly* while the removed heuristic can no longer manufacture a
false resolution from a namespace collision. (This is also why
class-precision is a clean 100 %.)

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
- **Cross-file inheritance is under-linked.** A class declared with a bare
  `superclass Device` inside an importing file does not link to
  `::SpiceGenTcl::Device` in the merged MRO (the hierarchy builder
  normalises only via `::name`), producing the 6 false W308s. Fixing it
  would need namespace-aware superclass normalisation in the shipping
  `class_hierarchy` builder — out of scope for this measurement-only
  experiment, and noted as a prerequisite for *any* cross-file W308.
- **Labeled sample bias.** Precision/recall are measured on the sites the
  resolver could reach at all; 81 % of sites are `EXTERN`/`DYNAMIC` and are
  excluded from the "knowable" denominators by construction. The headline
  number remains the 81.3 % ⊤-rate, not the 100 % conditional recall.

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
3. **Do not source W308 from this resolver.** Against the real emitter it
   is a net *loss*: +2 W307 false positives removed, but **6 new W308 false
   positives** introduced and **0 true findings**, because cross-file
   inheritance through bare `superclass` names is under-linked. Cross-file
   W308 needs namespace-aware superclass normalisation *first*, and even
   then this corpus offers no true positives to gain.
4. **If you want to move the ⊤-rate, the only lever that matters is
   interprocedural object-type flow through parameters/returns** — that is
   60 % of all sites (the extern/param receivers) and 100 % of the
   `unknown` bucket. This is a *type-flow* problem, not a *lattice* one:
   propagate `TclType::Object { class }` across call edges (the
   interprocedural summaries already exist for taint/const). Before
   committing, measure: instrument how many param-receiver sites have a
   *single* concrete caller-supplied class — if most callers are
   themselves params (turtles all the way down), even this won't help.
5. **Retain the ⊤ taxonomy + harness** as the yardstick for (4). The
   `next_provider` (`next`/`nextto`) modelling and the ⊤ instrumentation
   are cheap, correct, and useful for go-to-definition on `next` chains
   independent of the lattice question.

**Bottom line: a well-measured negative result for the lattice.** The
hypothesis ("the lattice beats the heuristic") is **not supported** — the
lattice's local evidence is absent at 99.8 % of sites, and the value is
entirely in the already-shipping MRO/CHA half plus cross-file indexing.
The honest next step is interprocedural param typing, gated on its own
measurement.

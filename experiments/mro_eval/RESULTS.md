# mro_eval — TclOO class-lattice dispatch resolver: results

Experiment for the two-halves TclOO model (MRO graph + object→class
lattice). Design: `docs/design/name-resolution.md` §5.6. Prototype:
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
# interprocedural ceiling + receiver taxonomy of the `unknown` ⊤ bucket
cargo run --release -p tcl-compiler --example mro_interproc -- $CORPUS
# method-override frequency (sizes the rename-across-overrides feature)
cargo run --release -p tcl-compiler --example mro_overrides -- $CORPUS
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
  taxonomy (`docs/design/name-resolution.md` §5.6).
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
| **`method_known` flag accuracy** | **104/107 = 97.2 %** | of resolved sites, the method flag agrees with whether the method really exists (was 94.5 % before the superclass-linking fix) |
| **abstention audit** | **41/41 correct** | every abstention had an `EXTERN` (param/global) or `DYNAMIC` truth — 0 abstentions on a knowable class |

Two things to read carefully:

- **100 % recall is over the *locally-knowable* subset, not all sites.**
  81.3 % of sites are `EXTERN`/`DYNAMIC` (param receivers, dynamic
  dispatch) and are *correctly* excluded from the knowable denominator.
  "Recall = 100 %" means the resolver never abstains on a site whose class
  a human could pin down locally — **not** that it resolves most sites.
- **The wrong `method_known` flags are false W308s**, not missed findings.
  The first cut had **6**, each a receiver whose class defines the method
  **by inheritance** (`superclass Device`/`Model` → `::SpiceGenTcl::Device`)
  but declared its superclass with a *bare* name the cross-file MRO builder
  left unlinked. **Fixed**: `class_hierarchy` now resolves a bare
  `superclass` via the defining class's namespace ancestry → global →
  globally-unique tail (see "Namespace-aware superclass linking" below),
  dropping the count to **3** and lifting method-flag accuracy 94.5 % →
  **97.2 %**. The remaining 3 are a *different* idiom —
  `method X {*}[info class definition <Other> X]`, a dynamically-computed
  method signature copied from another class, which method extraction does
  not register — a separate, narrower follow-up.

## W307/W308 delta vs. the shipping heuristic

`examples/mro_delta.rs` cross-tabulates the resolver's verdict against the
diagnostic the **real** analyser emits at each dispatch span (not a
reimplementation). Over the 1,803 sites:

| resolver \ shipping | W307 | W308 | none |
|---|--:|--:|--:|
| resolved-known | 2 | 0 | 333 |
| resolved-unknown | 3 | 0 | 0 |
| abstain | 46 | 0 | 1,414 |

(After the superclass-linking fix; before it, `resolved-unknown` was
`3 | 0 | 3`.) The shipping heuristic fires **51 W307 and 0 W308** at these
sites — it is already very conservative here. As a hypothetical
replacement the resolver would:

- **Remove 2 W307 false positives** (`resolved-known × W307`): it proves
  the head is a valid object dispatch with a known class + method.
- **Add 3 W308** (`resolved-unknown × W307`, reclassifying a vaguer W307) —
  but per the precision audit **all 3 are false positives** (the
  `{*}[info class definition …]` dynamic-method idiom). **0 true new
  findings.**
- **Regress nothing** (`abstain × W308 = 0`; it never contradicts a
  shipping W308).
- Leave 46 W307 untouched (`abstain × W307`) — no claim either way.

Net: **+2 real FP removed, 3 FP introduced, 0 TP gained** — still a net
loss, now driven only by the dynamic-method idiom rather than the
(now-fixed) inheritance gap. Another data point against sourcing W308 from
the resolver.

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

## Namespace-aware superclass linking (a shipped precision fix)

The precision audit surfaced 6 false W308s from bare `superclass Device`
names that the MRO builder (`class_hierarchy`) left unlinked to their
namespaced definitions — silently dropping inherited methods. **Fixed in
shipping** `class_hierarchy::build_supers_mixins_maps`: a bare superclass /
mixin name now resolves the way Tcl resolves the command —

1. in the **defining class's namespace**, walking outward to global; then
2. a **globally-unique simple-name** match (the `namespace import` idiom).

An ambiguous simple name (several classes share the tail) stays unlinked,
so no wrong edge is ever manufactured (unit-tested: ancestry link, unique
cross-namespace tail link, and ambiguous-tail abstention). This is a real
diagnostic improvement — it links cross-file inheritance the old `::name`
normalisation missed — and it *reduces* W308 false positives, so it is
safe to ship independently of the lattice question. Measured effect:
method-flag accuracy 94.5 % → 97.2 % (6 → 3 false W308s); the resolved
count is unchanged.

## Follow-up: where the ⊤-rate actually comes from

The recommendation names interprocedural parameter typing as the lever on
the 60 % `unknown` ⊤ bucket. `examples/mro_interproc.rs` measures that
ceiling — and **overturns the assumption**. Over the 1,083 `unknown`-⊤
sites:

| receiver origin | share | recoverable by the obvious pass |
|---|--:|---|
| proc/method **parameter** | **10 (0.9 %)** | fixpoint caller→param class propagation: **0** |
| **container iteration** (`foreach`/`lmap`/`dict for` over a collection) | **≥149 (≥13.8 %)** | lightweight element-typing: **0** |
| class instance variable | 10 (0.9 %) | constructor-binding: 0 |
| upvar alias | 0 | — |
| global / other | 914 (84.4 %) | — |

Two hard findings:

1. **Interprocedural *parameter* typing is a dead end here.** Parameters
   are 0.9 % of the residual, and a full caller→param class propagation to
   a fixpoint (the strongest form of the "obvious" pass) recovers **0**
   additional sites. The receivers simply are not proc parameters.
2. **The real residual is *container-of-objects iteration*.** The dominant
   identifiable pattern is `foreach elem [dict values $Container]` /
   `lmap x [dict values $Container]`, where `$Container` is an
   instance-variable dict/list of child objects (the idiomatic TclOO
   composite — `Circuit`→elements, `Device`→`Params`/`Pins`). Measured at
   ≥13.8 %, and this is a **lower bound**: the source scan only catches the
   var appearing as a loop variable in its own scope, so much of the
   84.4 % "global/other" is the same shape reached through
   `[dict get $C $k]` binds, nested scopes, and `my variable` indirection.

Resolving these needs **object-container element-typing**: infer that a
dict/list instance variable holds objects of class *C*, tracking dynamic
population (`dict append Params $k [::Ns::Parameter new …]`) through
`my variable` scoping and often large (analysis-guarded) method bodies.

**Prototyped and measured.** A source-level container element-typing pass
(scan `lappend`/`dict set`/`dict append`/`set … [list …]` for
`[Class new]` elements, key the container's element class, then type the
`foreach x [dict values $Container]` loop var) recovers **27 / 149
container-iteration receivers (18 %)** — but only **27 / 1,083 = 2.5 % of
the whole `unknown` ⊤ bucket**, i.e. it would move the overall ⊤-rate from
81.3 % to ~79.8 %. On the class-definition files alone (`SpiceGenTcl/src`)
it types 57 % of container receivers, but the population sites are often
cross-file or pass a `$var` element the syntactic pass can't type, and the
container-iteration share is itself a lower bound. So the ceiling is
**modest** even before accounting for the cost: this is a *materially
harder* analysis than the lattice (dynamic dict/list writes, `my variable`
scoping, guarded bodies), for a low-single-digit ⊤-rate improvement.

**Upshot:** the two "next levers" beyond the lattice are, respectively,
**refuted** (parameter typing: 0.9 % of ⊤, 0 recovered) and **low-ceiling +
expensive** (container element-typing: 2.5 % of ⊤ at its measured
prototype ceiling). This strengthens the negative recommendation: the cheap
wins are already shipping (MRO/CHA + workspace index), and the residual
⊤-rate is gated by container-of-objects modeling whose payoff is
small-and-costly, not by any machinery this experiment prototyped.

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

## Shipped editor improvements (the positive half)

The lattice is a negative result, but the *question* it framed — "what
more can the MRO/CHA half + workspace index give TclOO users?" — produced
concrete, sound-by-abstention wins that **do** ship. All are gated on the
already-validated MRO table + cross-file `all_classes` index, never on the
refuted lattice, and all abstain rather than guess:

- **Namespace-aware superclass linking** (`class_hierarchy.rs`). Resolves a
  bare `superclass Device` by walking the defining class's namespace
  ancestry → global → *globally-unique* tail, so cross-file inheritance
  links in the merged MRO. Removes 3 of the 6 false W308s the experiment
  surfaced (the rest need per-file import data); an ambiguous tail stays
  unlinked, never mis-linked.
- **W250 abstract-instantiation diagnostic** (`var_command.rs`). Flags
  `Foo new` / `Foo create o` / `set o [Foo new]` when `Foo.metaclass ==
  oo::abstract`. **0 false positives** on the corpus.
- **Type Hierarchy** super/subtypes (`type_hierarchy.rs`), same-document
  plus **cross-file** via the workspace class index (`classes_named` /
  `subclasses_of`), de-duplicated so the richer in-document item wins.
- **Hover**: MRO chain + direct subclasses on a class; inherited-from /
  overrides / provided-by notes on a method (`hover.rs`).
- **Method completion** including inherited methods, walking the MRO
  (`completion.rs`).
- **Go-to-definition on `next` / `nextto`** via `next_provider`
  (`definition.rs`) — the cheap, correct `next`-chain modelling the
  recommendation called out as worth keeping.
- **Rename across the override family** (`rename.rs` + `workspace_index.rs`
  + server). A method redefined up or down the hierarchy is one polymorphic
  name; renaming now spans the whole override-connected component instead of
  silently breaking the override. `examples/mro_overrides.rs` sizes it:
  **37.9 %** of direct method definitions sit in a cross-file override
  family, **17.8 %** within a single file. Both are now covered: the
  workspace index carries each class's defined-method names and resolves the
  **cross-file** override family (`method_override_family`, owner-aware super
  /mixin edges), and the server rewrites the declaration + resolvable call
  sites in every document that defines a family class — closing the ~20-point
  within-file → cross-file gap. Bounded by the analyser's single-document
  instance tracking: a `$obj method` site is rewritten only in a document
  that also defines the receiver's class (the same constraint under which the
  site resolves at all), so no resolvable site is left stale and no
  unresolvable one is guessed at.

These are the "cheap wins are already shipping" of the recommendation made
concrete: they extend reach for TclOO users without adding a single
confident-but-wrong resolution.

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
4. **The ⊤-rate lever is *container element-typing*, not parameter flow —
   and it is expensive. Decision: not productionized.** The follow-up
   experiment (above) refutes the parameter-typing hypothesis (0.9 % of ⊤;
   fixpoint recovers 0) and shows the residual is iteration over object
   dicts/lists held in instance variables. The prototype ceiling is already
   measured: **2.5 % of the `unknown` ⊤ bucket** (27 sites) — a
   low-single-digit ⊤-rate improvement for a *materially harder* analysis
   than the lattice (dynamic dict/list writes + `my variable` scoping +
   guarded-body handling). At that payoff-to-cost ratio it does **not** earn
   a place in shipping diagnostics or resolution now; the prototype stays in
   `class_lattice.rs` as measured, unwired. Revisit only if go-to-definition
   / hover on `foreach x [dict values $Container]` receivers becomes a named
   priority — the harness and ⊤ taxonomy are retained as the yardstick for
   that decision.
5. **Retain the ⊤ taxonomy + harnesses** as the yardstick for (4). The
   `next_provider` (`next`/`nextto`) modelling and the ⊤ instrumentation
   are cheap, correct, and useful for go-to-definition on `next` chains
   independent of the lattice question.

**Bottom line: a well-measured negative result for the lattice.** The
hypothesis ("the lattice beats the heuristic") is **not supported** — the
lattice's local evidence is absent at 99.8 % of sites, and the value is
entirely in the already-shipping MRO/CHA half plus cross-file indexing.
The honest next step is interprocedural param typing, gated on its own
measurement.

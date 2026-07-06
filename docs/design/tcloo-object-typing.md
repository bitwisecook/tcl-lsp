# TclOO object-type tracking — design & staged plan

Status: **active**. This note records the target architecture for tracking a
`TclOO` receiver's class (`$obj method …`, `[dict get $d $k] method …`, `my
method …`) and the staged path from today's provenance passes to it. It is
informed by a review of how modern OO / dynamic-language compilers do this
(sources at the end).

## The problem

Tcl is dynamically typed; an object is a command handle, dispatch is
`$obj method args`. To colour / complete / diagnose a method we need the
receiver's class(es). Today (issue #797 and its follow-ups) we resolve this with
a set of **provenance passes** feeding a highlight-only resolver:

- SSA type lattice: scalar `OBJECT(class)` and container `List/Dict<OBJECT(class)>`
  (`type_infer`).
- Doc-level union maps: `object_handle_classes`, `object_collection_classes`,
  interprocedural param/return, `my`-self, loop-var, cross-file `project_class_index`.

This works well for highlighting (see the `tcloo_dispatch_pattern_fixture`
golden test) but the interprocedural/field flow is an **unsound doc-level
union** (any var *named* `x` that is ever an object is treated as one
everywhere). That is fine for colour, wrong for diagnostics, and doesn't
compose. This note is the principled replacement.

## What the field does (and what to borrow)

- **CHA** (Dean/Grove/Chambers) — bound the callee set by the class hierarchy.
  We already have this: MRO + `method_providers`. Not applicable *alone* to a
  dynamic language (no declared type), but it's our fallback bound once we have
  a receiver class-set.
- **RTA** (Bacon/Sweeney) — intersect CHA with the set of classes actually
  *instantiated* in the program. **Cheapest high-value win**: one workspace
  "instantiated classes" query behind an incremental firewall.
- **VTA — Variable Type Analysis** (Sundaresan et al., OOPSLA'00) — **the
  primary model.** A flow-insensitive, context-insensitive *type-propagation
  graph*: nodes = variables / params / returns / **`(class, member)` field
  nodes** / **per-site container element nodes**; edges = assignments, field
  read/write, call actual→formal + return→result (to the CHA∩RTA callee set),
  container store/load; seed each `Class new` site with `{Class}`; solve by
  Tarjan-SCC collapse + topological propagation of **class-sets**. Sound,
  ~linear, field-sensitive-but-object-insensitive — the right economy. It maps
  1:1 onto our SSA + class index + `Dict<OBJECT>`.
- **Flow-typing** (TypeScript CF-narrowing, Sorbet, Typed Racket occurrence
  typing) — intraprocedural, flow-sensitive narrowing over the CFG, boundaries
  via signatures, **abstain to `⊤`** rather than guess. SSA gives us the
  flow-sensitivity for free; narrow on `[info object class $x]` / `[$x isa C]`.
- **Incremental firewall** (rust-analyzer ItemTree/body split, salsa
  durability) — cross-procedure type flow travels **only through per-method
  summaries**, never raw bodies; a body edit that doesn't change the boundary
  summary early-cuts-off and nothing downstream reruns.
- **Not building**: Andersen/Steensgaard points-to, k-CFA / full
  object-sensitivity heap cloning, per-object field modelling, whole-program
  closed-world fixpoints — sound but neither incremental nor worth it for an
  editor over a language where half the values legitimately resolve to `⊤`.

## Lattice

Generalise the receiver type from a single class to a **class-set**:

```
⊥ (unreachable)  <  OBJECT({A})  <  OBJECT({A,B,…})  <  ⊤ (any handle)
```

- **join = set union.** Two distinct concretes widen to a *set*, **not** to the
  least-common-ancestor (LCA loses the concrete implementers devirtualisation
  needs, and is unsound as a *value* claim). Represent as an interned sorted
  small set + a `Top` sentinel.
- **widen to `⊤`** past a cardinality cap `k` (≈ 4–8), and immediately on
  *escape* — a value through `eval` / `uplevel` / a dynamic receiver / an
  unmodelled command.
- **dispatch resolves on the MRO-induced *provider* set**: map each class in the
  set through the MRO to its provider of the selector, union the providers.
  A 3-element type set frequently collapses to a 1-element provider set — that
  is the devirtualisation, and it is more precise than reasoning on types.

## Staged plan

Each stage is independently landable, tested, and measured against the
committed fixture (`tcloo_dispatch_pattern_fixture`) + the corpus resolution-rate
experiment (`experiments/tcloo_dispatch`).

- **Stage 0 — lattice class-set.** `OBJECT(class)` → `OBJECT(class-set)` in
  `TypeLattice`; union join + cardinality widening + escape-to-`⊤`. The current
  single-class element becomes the singleton case; replaces the unsound
  doc-union collapse with a sound widening join.
- **Stage 1 — CHA∩RTA.** A workspace "instantiated classes" query; resolve on
  the provider set intersected with instantiated reachability. First real
  precision result; inherently incremental (coarse global input behind a
  firewall).
- **Stage 2 — intra-body flow-typing.** SSA-based narrowing on guards
  (`info object class`, `isa`), φ-joins do the merges. Powers hover / completion
  / semantic tokens with no interprocedural cost.
- **Stage 3 — VTA graph.** Type-propagation graph over SSA + params/returns +
  `(class, member)` fields + per-site container elements; seed from `new`;
  SCC-collapse + topo solve. The principled replacement for the doc-level union.
- **Stage 4 — incremental firewall.** Per-method **summary** query
  `(receiver+param class-sets) → (return, written-field class-sets)`, keyed on
  the body; workspace propagation composes summaries + class index +
  instantiated-set. rust-analyzer's ItemTree/body split; class index durable,
  bodies volatile.
- **Stage 5 — stubs & abstention.** Signature stubs for `oo::` builtins & common
  libs so boundary flow doesn't hit `⊤` at every call; everywhere unprovable,
  abstain to `⊤` and emit no receiver-dependent highlight/diagnostic.

Highlight-first means: prefer silence over a wrong narrowing at every stage.

## Evidence: what the corpus actually needs (`tcloo_diag`)

Stage 3's first increment landed as a **VTA-lite fixpoint**
(`object_types::propagate_object_flow`): a name-keyed union-join propagation
graph over assignment (aliasing), proc-return, proc-parameter, and
constructor-parameter edges. It is sound and resolves the dependency-injection
shape (object → constructor → instance variable → dispatch), but the
`tcloo_diag` provenance experiment showed it does **not** move the corpus
resolution rate — because the corpus bottleneck is elsewhere. Categorising the
8241 unresolved `$var` receivers:

- **71% "unbound"** — dominated by **snit** (`$self` = 1035, `my…` components,
  `$hull`, `options(...)`) and **Tk widget paths** (`$win.c`), not `TclOO`.
- ~12% within-CU `TclOO` edges (alias / method-return / proc-return /
  collection) — the slice this and prior stages address.
- ~11% `cmd-return` — mostly non-object commands that should *abstain*.

So the measured priority order (highest corpus impact first) is:

1. **snit dialect model** — `$self` → enclosing snit type (the snit analogue of
   `TclOO`'s `my`, already handled), components via `install`/`delegate`,
   `$hull`. This is the data-driven dispatch model of Stage/Phase 3 and the
   single biggest lever (~1500+ receivers). **`$self` / `$this` self-dispatch has
   landed** (snit + itcl): `definer_class_name` recognises the snit/itcl definer
   shape so `enclosing_class` is threaded into their bodies, and the self-call
   resolver accepts `$self` / `$this` heads. Measured effect: overall dispatch
   resolution ~doubled (local 6.8%→13.6%, project 8.1%→14.8%; see
   `experiments/tcloo_dispatch/RESULTS.md`). **Named-constructor typing
   (`set o [foo create x]`) has also landed** — the signature scan records snit
   types as classes so the receiver types `OBJECT(class)`. Remaining snit slices:
   components (`install`/`delegate`), `$hull`, bareword named-object commands.
2. **Cross-file object provenance** — a workspace union of the handle /
   collection maps, mirroring `project_class_index`, for the cross-file half of
   `param` / `unbound`.
3. **Method return-type summaries** — `set x [$typed m]` / `[my m]` where `m`
   returns an object (Stage 3/4 interprocedural summaries).
4. **Signature stubs & explicit abstention** (Stage 5) — deflate the
   `cmd-return` denominator so the rate reflects *typable* receivers.

The lesson for the staged plan: the stages are individually correct, but their
*ordering* should follow the provenance histogram — snit and cross-file first.
See `experiments/tcloo_diag/RESULTS.md`.

## Sources

CHA/RTA (Dean/Grove/Chambers ECOOP'95; Bacon/Sweeney OOPSLA'96); VTA
(Sundaresan et al., *Practical Virtual Method Call Resolution for Java*,
OOPSLA'00); Andersen vs Steensgaard pointer analysis; Smaragdakis et al., *Pick
Your Contexts Well* (object-sensitivity), POPL'11; TypeScript control-flow
narrowing; Flow (*Fast and Precise Type Checking for JavaScript*); Sorbet
flow-sensitive typing; Typed Racket occurrence typing (Tobin-Hochstadt &
Felleisen, ICFP'10); rust-analyzer incrementality (ItemTree/body firewall,
salsa durability).

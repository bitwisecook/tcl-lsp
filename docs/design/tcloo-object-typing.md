# TclOO object-type tracking

The architecture for tracking a `TclOO` receiver's class — `$obj method …`,
`[dict get $d $k] method …`, `my method …` — and how it relates to the
provenance passes that feed it. The approach is grounded in how modern OO
and dynamic-language compilers solve the same problem; sources at the
end.

## The problem

Tcl is dynamically typed; an object is a command handle, and dispatch is
`$obj method args`. To colour, complete, or diagnose a method call, we need
the receiver's class or classes.

The resolver that ships today is fed by a set of **provenance passes** and
is deliberately highlight-only, because its interprocedural and field flow
is an unsound document-level union. The [current
implementation](#the-current-implementation) section below says exactly
what exists; the [Lattice](#lattice) section says what a sound
replacement looks like.

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

## The current implementation

Two layers exist today, and they answer different questions.

**The shipping layer** is the set of provenance passes plus the
highlight-only resolver:

- The SSA type lattice carries scalar `OBJECT(class)` and container
  `List/Dict<OBJECT(class)>` (`rust/tcl-compiler/src/type_infer.rs`).
- `rust/tcl-compiler/src/object_types.rs` owns the document-level union maps
  — `object_handle_classes`, `object_collection_classes` — plus
  `propagate_object_flow`, a VTA-lite name-keyed union-join fixpoint over
  assignment (aliasing), proc-return, proc-parameter, and
  constructor-parameter edges. It is sound and it resolves the
  dependency-injection shape: object → constructor → instance variable →
  dispatch.
- `project_class_index` supplies the cross-file class view.
- `definer_class_name` recognises the snit and itcl definer shapes, so
  `enclosing_class` is threaded into their bodies and the self-call
  resolver accepts `$self` / `$this` heads. Named-constructor typing
  (`set o [foo create x]`), `install NAME using TYPE` component typing (a
  source scan, since snit bodies are not lowered), and the bare-word
  constructor (`set c [Type inst]`, gated on `Type` being a known
  snit-family class whose first argument is not a typemethod) all resolve
  through the same path.

The document-level union is an **unsound over-approximation**: any variable
*named* `x` that is ever an object is treated as one everywhere. That is
acceptable for colour, wrong for diagnostics, and does not compose. It is
why the shipping layer drives highlighting, hover, and completion but not
receiver-dependent diagnostics.

**The prototype layer** is `rust/tcl-compiler/src/analyser/class_lattice.rs`
— the class-set lattice plus a precomputed per-class MRO graph, described
above. It is explicitly not wired into any diagnostic; nothing under
`analyser/diagnostics/` calls into it. It exists to measure whether
splitting the problem into an MRO graph plus a binding lattice over the
existing SSA beats the current heuristic on real code at an acceptable
⊤-collapse rate. See `experiments/mro_eval/RESULTS.md`.

Everything else in the model above — CHA∩RTA over a workspace
instantiated-classes query, SSA guard narrowing on `info object class` /
`isa`, per-method summaries behind an incremental firewall, signature
stubs for `oo::` builtins — is design, not code.

## Evidence: where the remaining gap is (`tcloo_diag`)

The `tcloo_diag` provenance experiment categorised 8241 unresolved `$var`
receivers across the corpus, and the result is the reason the VTA-lite
fixpoint did not move the resolution rate: **the bottleneck is not
TclOO.**

- **71% "unbound"** — dominated by **snit** (`$self` = 1035, `my…`
  components, `$hull`, `options(...)`) and **Tk widget paths** (`$win.c`),
  not `TclOO`.
- ~12% within-CU `TclOO` edges (alias, method-return, proc-return,
  collection) — the slice the shipping layer addresses.
- ~11% `cmd-return` — mostly non-object commands that should *abstain*.

Landing the snit self-dispatch and constructor slices moved the corpus
resolution rate from 6.8% local / 8.1% project to **14.0% local / 16.4%
project** (see `experiments/tcloo_dispatch/RESULTS.md`), which is what
confirms the histogram rather than the type theory should drive the order
of work here. Tk widget paths — the other half of the "unbound" mass — are
handled by their own model; see
[`tk-widget-instance-typing.md`](tk-widget-instance-typing.md).

The categories still open, in corpus-impact order:

1. **Remaining snit slices** — `$hull` (usually a Tk widget) and bareword
   named-object commands.
2. **Cross-file object provenance** — a workspace union of the handle and
   collection maps, mirroring `project_class_index`, for the cross-file
   half of `param` / `unbound`.
3. **Method return-type summaries** — `set x [$typed m]` / `[my m]` where
   `m` returns an object.
4. **Signature stubs and explicit abstention** — deflating the
   `cmd-return` denominator so the rate reflects *typable* receivers.

## Sources

CHA/RTA (Dean/Grove/Chambers ECOOP'95; Bacon/Sweeney OOPSLA'96); VTA
(Sundaresan et al., *Practical Virtual Method Call Resolution for Java*,
OOPSLA'00); Andersen vs Steensgaard pointer analysis; Smaragdakis et al., *Pick
Your Contexts Well* (object-sensitivity), POPL'11; TypeScript control-flow
narrowing; Flow (*Fast and Precise Type Checking for JavaScript*); Sorbet
flow-sensitive typing; Typed Racket occurrence typing (Tobin-Hochstadt &
Felleisen, ICFP'10); rust-analyzer incrementality (ItemTree/body firewall,
salsa durability).

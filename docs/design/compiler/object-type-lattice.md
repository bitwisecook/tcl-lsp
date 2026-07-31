# Object-type lattice — the object-handle → class carrier

The contract for `rust/tcl-compiler/src/object_types.rs` and the
`AnalysisResult::object_handle_facts` carrier it fills. This is the fact
behind every "`$obj method …` resolves to class `C`" answer the LSP gives:
semantic tokens, go-to-definition, find-references, rename safety, the
W307 / W308 diagnostics, and the optimiser's devirtualisation.

Issue #994 is the unification this doc records the shape of; the staged
landing is C5a (this carrier, no consumer), C5b (the five dispatch
consumers plus tokens), C5c (the cross-document index facts of #1099).
Cost measurement for C5a: `experiments/object_lattice/RESULTS.md`.

## §0 — Four maps, four different keys

Before the unification there were **four** independently-produced answers to
"what class does this name hold?", and they could disagree on the same
document:

| map | produced by | key | scope | consumers |
|---|---|---|---|---|
| `AnalysisResult::instance_classes` | the analyser's syntactic walk, settled late from the CU (`analyser/diagnostics.rs`, `settle_pending_instance_class_sites`) | bare name | last-write-wins across the file | find-references, rename, definition ×2, code lens, W307 / W308 |
| `object_handle_classes` | the VTA-lite lattice over the `CompilationUnit` | bare name | union across the file | optimiser, `compilation_unit`, `interprocedural`, `type_infer`, semantic tokens |
| `object_collection_classes` | the SSA type lattice's container element-typing | bare name | union across the file | collection dispatch (`[dict get $pins $k] m`) |
| `AnalysisResult::instance_command_bindings` | the analyser's `CLASS create NAME` sites | **namespace-qualified** command name | per creation site | the #981 namespace-scoped object-command path |

The failure mode this shape produces is structural, not incidental: tokens
read the lattice, navigation reads `instance_classes`, so the same
`$obj method` could be coloured as a resolved dispatch and simultaneously
have no definition to jump to. Every precision fix had to be made — and
kept in sync — in up to four places.

`instance_command_bindings` is deliberately **not** folded into the
lattice: it is keyed by qualified command name precisely because the
name-keyed maps cannot tell `::a::rex` from `::b::rex`, and reading a
name-keyed map on that path is the #981 bug.

## §1 — The carrier

`object_handle_facts(cu, registry) -> ObjectHandleFacts`, produced **once**
per analysis at `analyser/diagnostics.rs`'s CU-derived fact seam
(`settle_cu_derived_object_facts`), which both the whole-file and the
per-item incremental path reach.

| field | key | contents |
|---|---|---|
| `any_scope` | bare name | the pre-existing union — `object_handle_classes` verbatim |
| `by_scope` | `(owner_qualified_name, name)` | the same bindings, attributed to the unit the binding edge binds in |
| `owner_spans` | sorted by `(start, end)` | `(span, unit, class?)` per proc / method, for `owner_at(offset)` |
| `collections` | bare name | `object_collection_classes` verbatim |
| `returns_object` | proc qualified name | the factory-return class (previously computed inside the fixpoint and discarded) |
| `global_object_cells` | `::`-qualified name | the `::`-qualified subset of `any_scope` |

### Owner attribution

The owner is *where the name lives*, which for each VTA edge is:

| edge | example | owner |
|---|---|---|
| seed (harvest) | `set c [Chart new]` | the harvesting unit |
| aliasing | `set b $a` | the assigning unit |
| proc return | `set q [make]` | the assigning unit |
| proc parameter | `connect $p` → `dev` | the **callee** (`::connect`) |
| constructor parameter | `Wrap new $p` → `inner` | the **constructor** (`::Wrap::<constructor>`) |

…with one override: a name that is a **class instance variable** is owned
by the **class**, unioned across its methods. That union is not a
convenience — it is the interprocedural bridge issue #797 needs. An object
built into `Pins` in `Device::add` and dispatched from `Pins` in
`Device::use` is connected by exactly nothing else: no intraprocedural
lattice can join two method bodies, and `mro_eval` measured 99.8 % ⊤
intraprocedurally on real TclOO corpora. Keying instance variables by class
reproduces the bridge while still refusing to merge a `chart` local in one
proc with a `chart` local in another.

### Scoped propagation, not just scoped keying

Owner attribution decides where a binding is *written*. It is not enough on
its own: the edge's **source** must be resolved in the scope that owns it
too, or the narrow map inherits the wide map's collisions. After

```tcl
proc a {} { set x [Pin new] }
proc b {} { set x 0; set y $x }
```

resolving `b`'s `set y $x` against the union would find `a`'s `x` and record
`(::b, y) → ::Pin` — a **false singleton** in the map C5b's rename edits and
the "provably a different class" refusal gate treat as authoritative. That is
the R2 wrong-rewrite hazard `by_scope` exists to prevent, so the fixpoint
resolves each edge's source through `by_scope` itself:

| edge | source | resolved in |
|---|---|---|
| aliasing | the read variable | the **reading unit** (then its class, for an instance variable; then nothing) |
| proc return | the callee's return type | nowhere — it is not a variable read, so it is unit-independent |
| proc parameter | the call-site argument | the **caller**, bound in the callee |
| constructor parameter | the call-site argument | the **caller**, bound in the constructor |

The two lower rows are genuine cross-scope flows and are unaffected: what is
excluded is only a name-keyed variable *read* resolving through another
unit's binding.

Both facts advance in one walk, each reading back only its own map, so
`any_scope` stays exactly the union it has always been — the module's
documented highlight-grade heuristic, which semantic tokens and the existing
compiler consumers still read.

`classes_in_scope(offset, var)` implements the consumer-side lookup: the
owning unit's key first, the enclosing class's key second.
`owner_spans` carries one entry per proc and method plus a whole-file entry
for `::top`, so a top-level handle has an owner too. Synthetic **body
units** (`apply` lambdas, `namespace eval` blocks) deliberately get no
entry: the harvest does not visit them at all, so a byte inside one
resolves to its enclosing owner rather than to a scope the lattice knows
nothing about.

### Soundness directions

Every map is **best-effort**. An absent key means *no evidence in this
document*; it is never proof that a name holds no object. The empty default
is what every CU-less path produces (structure-only analysis, a panicking
CU build), so a consumer must degrade to its pre-lattice behaviour rather
than conclude anything from emptiness.

`by_scope` is the *narrow* map, `any_scope` the *wide* one. Which to read is
decided by what a wrong answer costs:

| consumer | map | why |
|---|---|---|
| rename edits, find-references | `by_scope` only | widening rewrites an unrelated variable — a wrong edit, not a missed one (and see "Scoped propagation" above: the narrowness must survive the fixpoint, not just the key) |
| definition, type-definition, hover | `by_scope`, then `any_scope` labelled as a guess | a wrong jump is recoverable; a missing one is not |
| semantic tokens / highlighting | either | colour-only, and the union is what shipped before |
| "provably a *different* class, so refuse/skip" gates | `by_scope` singletons only | widening turns an abstention into a false certainty, silencing a refusal that protects the user |

The invariant behind the last row, from the #994 design: **an abstention
must never become "provably not family"**. Documented abstentions —
dict/list-carried and callback-registered handles, computed array indices,
interp boundaries, unqualified globals — are all backstopped by the
untracked-receiver refusal.

## §2 — How the lattice is built

1. **Harvest** every unit (top level, procs, methods): syntactic
   `set VAR [Class new|create …]` assignments, registry naming factories
   (`struct::graph myG`, and Tk widget paths once the registry declares
   them — #927), and every SSA value the type lattice typed `OBJECT(class)`
   (which is where collection retrievals like `set p [dict get $pins $k]`
   enter).
2. **Propagate** along the four VTA edges (Sundaresan et al., OOPSLA'00) to
   a bounded 6-round fixpoint. Nodes are name-keyed — field-based and
   object-insensitive, VTA's economy — and the join is set union.
   Convergence is decided by `any_scope` alone, so the scope-keyed twin can
   never change the number of rounds or the union's contents.

### The empty-seed fast path

The propagation's original early-out tested the **callee-side** maps
(returns / proc params / ctor params), which are non-empty for any file that
merely defines a proc — so every ordinary non-OO file paid a full statement
walk to discover it had nothing to propagate. The fast path skips the walk
when no edge can fire, which needs **three** conditions, not one:

- no seeded handle (kills every `out`-driven edge), **and**
- no procedure returns an object (kills the proc-return edge), **and**
- no argument could be a bracketed registry constructor (kills
  `arg_classes`' direct-constructor branch, which reads no seed at all).

Dropping the second or third condition is a real regression, not a
theoretical one: `proc make {} { return [Pin new] }; set c [make]`,
`take [listbox .l]`, `Wrap new [listbox .l]`, and `take [struct::graph]`
each bind from an empty seed set. `object_handle_classes_full_walk` keeps
the pre-fast-path walk available so the unit test can pin the equality, and
`experiments/object_lattice/RESULTS.md` measures what the gate saves
(109 of 154 corpus files skip the walk; 55 % of their lattice time).

## §3 — What the lattice deliberately does not do

- It does not replace `instance_classes`. Literal replacement destroys the
  ambiguity signal (`ambiguous_instance_names`), changes W307 / W308 / W123,
  and breaks the per-item `extend` merge.
- It is not consulted per-consumer with per-consumer precedence rules. Five
  independent precedence rules drift; the carrier exists so all five
  dispatch sites read **one** fact through **one** accessor.
- It does not bind the `= | := | as | deserialize` operator words a
  `struct::graph = $serial` deserialise form puts in the name slot. That
  abstention holds in **both** maps: a bogus `=` handle would suppress a
  real W123 / W307 and, once C5b's consumers read `by_scope`, would
  mis-resolve a command literally named `=`.
- It does not make the fixpoint cleverer. On 66,827 lines of real TclOO the
  four propagation edges fired 3 times against 86 harvest seeds; the value
  is in the carrier being shared, not in the propagation.

## See also

- [`cfg-ssa-fact-model.md`](cfg-ssa-fact-model.md) — the fact model the
  lattice reads (`FunctionUnit::types`, `return_type`).
- [`compilation-unit-contracts.md`](compilation-unit-contracts.md) — the
  unit the lattice rides on and its incremental cache expectations.
- [`interprocedural-analysis.md`](interprocedural-analysis.md) — the
  `ObjectTypeMap` consumer of `object_handle_classes`.
- `experiments/object_lattice/RESULTS.md` — C5a's cost gate (M1).
- `experiments/mro_eval/RESULTS.md` — why the cross-file class index, not
  the intraprocedural lattice, is where dispatch resolution comes from.

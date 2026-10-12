# Object-type lattice — the object-handle → class carrier

The contract for `rust/tcl-compiler/src/object_types.rs` and the
`AnalysisResult::object_handle_facts` carrier it fills. This is the fact
behind every "`$obj method …` resolves to class `C`" answer the LSP gives:
semantic tokens, go-to-definition, find-references, rename safety, the
W307 / W308 diagnostics, and the optimiser's devirtualisation.

Cost measurement: `experiments/object_lattice/RESULTS.md`.

## §0 — Four maps, four different keys

Four independently-produced answers to "what class does this name hold?"
exist, and read separately they can disagree on the same document:

| map | produced by | key | scope | consumers |
|---|---|---|---|---|
| `AnalysisResult::instance_classes` | the analyser's syntactic walk, settled late from the CU (`analyser/diagnostics.rs`, `settle_pending_instance_class_sites`) | bare name | last-write-wins across the file | find-references, rename, definition ×2, code lens, W307 / W308 |
| `object_handle_classes` | the VTA-lite lattice over the `CompilationUnit` | bare name | union across the file | optimiser, `compilation_unit`, `interprocedural`, `type_infer`, semantic tokens |
| `object_collection_classes` | the SSA type lattice's container element-typing | bare name | union across the file | collection dispatch (`[dict get $pins $k] m`) |
| `AnalysisResult::instance_command_bindings` | the analyser's `CLASS create NAME` sites | **namespace-qualified** command name | per creation site | the #981 namespace-scoped object-command path |

Tokens read the lattice and navigation reads `instance_classes`, so the same
`$obj method` could be coloured as a resolved dispatch and simultaneously
have no definition to jump to; the carrier exists so every precision fix is
made once.

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
| `any_scope` | bare name | the advisory union — `object_handle_classes` verbatim |
| `by_scope` | `(owner_qualified_name, name)` | advisory candidate bindings, attributed to the unit the binding edge binds in |
| `proven_reads` | original variable-substitution span and spelling | actual object allocation, class incarnation and OO dispatch generation, retained through argument evaluation |
| `owner_spans` | sorted by `(start, end)` | `(span, unit, class?)` per proc / method, for `owner_at(offset)` |
| `collections` | bare name | `object_collection_classes` verbatim |
| `returns_object` | proc qualified name | the factory-return class |
| `global_object_cells` | `::`-qualified name | the `::`-qualified subset of `any_scope` |

### Owner attribution

The owner is *where the name lives*, which for each VTA edge is:

| edge | example | owner |
|---|---|---|
| seed (harvest) | `set c [Chart new]` | the harvesting unit |
| aliasing | `set b $a` | the assigning unit |
| proc return | `set q [make]` | the assigning unit |
| method return (#1143) | `set b [$a make]` | the assigning unit |
| proc parameter | `connect $p` → `dev` | the **callee** (`::connect`) |
| constructor parameter | `Wrap new $p` → `inner` | the **constructor** (`::Wrap::<constructor>`) |

The method-return edge fires only for a *directly-declared* method of a
receiver class the lattice already tracks, whose own inferred return type
names an object class (`::A::make` → `::B`), and only for a bareword method
word — a computed member (`[$a $m]`) proves nothing.  A receiver class that
merely inherits the method resolves nothing (the lattice carries no MRO),
so the edge abstains there.

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
`(::b, y) → ::Pin` — a candidate attributed to the wrong unit. Scoped
propagation prevents that collision by resolving each edge's source through
`by_scope` itself. It still unions different contents versions and does not
prove the runtime receiver; edits and refusal gates require `proven_reads`.

| edge | source | resolved in |
|---|---|---|
| aliasing | the read variable | the **reading unit** (then its class, for an instance variable; then nothing) |
| proc return | the callee's return type | nowhere — it is not a variable read, so it is unit-independent |
| proc parameter | the call-site argument | the **caller**, bound in the callee |
| constructor parameter | the call-site argument | the **caller**, bound in the constructor |

The two lower rows are genuine cross-scope flows and are unaffected: what is
excluded is only a name-keyed variable *read* resolving through another
unit's binding.

Both facts advance in one walk, each reading back only its own map.
`any_scope` is the module's documented highlighting heuristic; it supplies
candidate assistance rather than a runtime class proof.

`classes_in_scope(offset, var)` queries the original variable-read span and
its actual allocation and current dispatch receipt. It never falls back to a scope union.
`owner_spans` retains procedure, method and top-level ownership for candidate
assistance; that attribution alone cannot establish a runtime value inside a
synthetic body or after a later write.

### Soundness directions

Every map is **best-effort**. An absent key means *no evidence in this
document*; it is never proof that a name holds no object. The empty default
is what every CU-less path produces (structure-only analysis, a panicking
CU build), so a consumer must abstain from a runtime class claim when the
positioned proof is missing.

`by_scope` narrows candidate ownership but still unions different SSA versions.
It cannot prove what a variable contains at a later dispatch. For example,
`set x [C new]; set x 0; set y $x` must not reuse the earlier object value.
`proven_reads` requires the shared source owner's object-instance receipt and
validates it again at the dispatch after argv. An unchanged SSA version alone
cannot preserve the object's class: `oo::objdefine $object class B` changes
the dispatch without changing the variable, and an argument callback can do
the same after the receiver read. Missing or conflicting allocation, class
incarnation or dispatch receipts remain unknown. `classes_in_scope` queries
this positioned proof despite its compatibility name. Physical SSA type-read
receipts remain separate advisory candidates.

| consumer | map | why |
|---|---|---|
| rename edits, find-references | `proven_reads` only | each receiver needs its actual reaching object value |
| definition, type-definition, hover | `proven_reads`; advisory candidates may be labelled as a guess | a wrong jump is recoverable; a missing one is not |
| semantic tokens / highlighting | either | colour-only candidate assistance |
| "provably a *different* class, so refuse/skip" gates | `proven_reads` singletons only | widening turns an abstention into a false certainty, silencing a refusal that protects the user |

The invariant behind the last row: **an abstention
must never become "provably not family"**. Documented abstentions —
dict/list-carried and callback-registered handles, computed array indices,
interp boundaries, unqualified globals — are all backstopped by the
untracked-receiver refusal.

## §2 — How the lattice is built

1. **Harvest** every unit (top level, procs, methods): syntactic
   `set VAR [Class new|create …]` assignments, registry naming factories
   (such as `struct::graph myG`), and every SSA value the type lattice typed `OBJECT(class)`
   (which is where collection retrievals like `set p [dict get $pins $k]`
   enter).
2. **Propagate** along the four VTA edges (Sundaresan et al., OOPSLA'00) to
   their finite monotone fixpoint. Nodes are name-keyed — field-based and
   object-insensitive, VTA's economy — and the join is set union.
   Both candidate maps participate in convergence; no fixed round count truncates
   a longer finite chain. Runtime read proofs come independently from the source owner's preserved object allocation and dispatch receipts.

### The empty-seed fast path

The propagation skips its statement walk when no edge can fire, which needs
**three** conditions, not one:

- no seeded handle (kills every `out`-driven edge), **and**
- no procedure returns an object (kills the proc-return edge), **and**
- no argument could be a bracketed registry constructor (kills
  `arg_classes`' direct-constructor branch, which reads no seed at all).

`proc make {} { return [Pin new] }; set c [make]`, `take [listbox .l]`,
`Wrap new [listbox .l]`, and `take [struct::graph]` each bind from an empty
seed set, so dropping the second or third condition is a regression.
`object_handle_classes_full_walk` keeps the unconditional walk available so
the unit test can pin the equality; `experiments/object_lattice/RESULTS.md`
measures what the gate saves.

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
  real W123 / W307 and would mis-resolve a command literally named `=`.
- It does not make the fixpoint cleverer. On 66,827 lines of real TclOO the
  four propagation edges fired 3 times against 86 harvest seeds; the value
  is in the carrier being shared, not in the propagation.

## §4 — Consumers

Every dispatch consumer reads the lattice through **one** accessor pair in
`tcl-lsp-core/src/definition.rs`:

- `receiver_instance_class_at(analysis, receiver, is_dollar, offset)` —
  `instance_classes` first, then, for a `$var` receiver only,
  `lattice_singleton_class`: the **singleton** class
  `proven_reads` proves at the substitution containing `offset`. A
  multi-class binding abstains — every caller edits or navigates, so a
  guess is a wrong edit, not a missed one.
- Consumers: go-to-definition (`instance_method_definition`), hover,
  completion (both the prefix and fuzzy paths), find-references
  (`instance_method_references`), prepare-rename / rename
  (`method_target_with_access` and the `$obj method` rename tier).

Two scan-shaped consumers read the same fact per site rather than per
cursor:

- The `$v method` call-site scan (`find_obj_method_call_sites`) matches a
  site when `instance_classes` binds `v` **or** its `proven_reads` singleton at
  the site's own offset is a class whose instances dispatch the method
  (`lattice_dispatch_family`).  Find References, rename edits, the code
  lens count, and the lens click all go through this one scanner.
- The rename refusal gate (`rename_safety::dispatch_hazard`) resolves an
  otherwise-untracked receiver through the same singleton: in-family →
  covered by the scan (no hazard); provably another class → no hazard;
  absent or multi-class → the untracked-receiver refusal stands.

On the compiler side the same facts feed the diagnostics, which is what
pins the "tokens and navigation can never disagree with W307" invariant
(issues #1143, #1200):

- `live_instance_at_dispatch` requires the exact source object allocation,
  class incarnation and dispatch generation retained after argv. W308 checks
  only absence from that instance's immutable manufacture-time method inventory;
  document-final method tables and advisory class labels provide no authority.
  Custom unknown handlers or unsupported inheritance leave that inventory open.
  Known private names suppress an absence warning without proving public visibility
  or a method signature.
- `emit_cmd_command_diagnostics` types a `[make]` head from
  `returns_object`, and a bare object-producing substitution head with no
  method word is E001 (`e001_for_bare_cmd_dispatch`) under the same
  locally-known-TclOO gate as the `$var` form.

## See also

- [`cfg-ssa-fact-model.md`](cfg-ssa-fact-model.md) — the fact model the
  lattice reads (`FunctionUnit::types`, `return_type`).
- [`compilation-unit-contracts.md`](compilation-unit-contracts.md) — the
  unit the lattice rides on and its incremental cache expectations.
- [`interprocedural-analysis.md`](interprocedural-analysis.md) — the
  `ObjectTypeMap` consumer of `object_handle_classes`.
- `experiments/object_lattice/RESULTS.md` — the carrier's cost gate.
- `experiments/mro_eval/RESULTS.md` — why the cross-file class index, not
  the intraprocedural lattice, is where dispatch resolution comes from.

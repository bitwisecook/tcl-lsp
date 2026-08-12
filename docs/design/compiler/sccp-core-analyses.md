# SCCP and core analyses (Stage 6)

How sparse conditional constant propagation, liveness, and the type lattice
work together over the SSA graph, and why a value that looks constant can
still settle at `OVERDEFINED`.

`sccp()` in `sccp.rs` runs SCCP (Sparse Conditional Constant Propagation) over
the SSA graph, producing an `SccpResult` with the per-value lattice, the
executable blocks and edges, and the constant branches.  Type information,
liveness, dead stores, read-before-set, and unused variables are produced by
separate passes and reach consumers through the per-function `FunctionUnit`.

> **`FunctionAnalysis` is not on the live path.**  The aggregate named
> throughout this document is declared in `rust/tcl-compiler/src/analyses.rs`,
> but nothing builds, returns, or reads one — its only construction is
> `::default()` inside that module's own tests, and there is no
> `analyse_function()`.  Read `FunctionAnalysis.X` below as "the X fact",
> whose live home is `FunctionUnit` (`sccp`, `types`, `taints`, `def_use`,
> `memory_ssa`, `rendered_props`) or a pass-local return value such as
> `liveness_dead_stores()`'s `Vec<DeadStore>`.  Issue #1406 tracks the gap.

Source: `rust/tcl-compiler/src/sccp.rs` (`sccp`, `SccpResult`),
`rust/tcl-compiler/src/analyses.rs` (the lattice types),
`rust/tcl-compiler/src/compilation_unit.rs` (`FunctionUnit`),
`rust/tcl-compiler/src/types.rs`

### SCCP — constant propagation

The SCCP value lattice:

```
Unknown  ──►  Const(v)  ──►  ConstSet([v…])  ──►  Overdefined
(bottom)   (provably const)  (≤ 32 values)   (top / anything)
```

`LatticeValue` and its `ConstValue` payload (`Int` / `Float` / `Bool` /
`String`) live in `analyses.rs`; `sccp::join` is the meet operator.
`ConstSet` is the finite-value-set kind between `Const` and `Overdefined`,
capped by `analyses::MAX_CONSTSET_SIZE` (32) — a union past the cap widens
to `Overdefined`, and a union that collapses to one value falls back to
`Const`.

SCCP walks the SSA graph and propagates:
- `Statement::AssignConst { value: "42", .. }` → `Const(Int(42))`
- `Statement::AssignValue { value: "${x}", .. }` where `x₁ = Const(Int(42))` → `Const(Int(42))`
- Phi nodes: `join(Const(42), Const(42))` → `Const(42)`
- Phi nodes: `join(Const(42), Const(99))` → `ConstSet([42, 99])`, widening to `Overdefined` past the set cap
- Loop-carried values: `Overdefined` (value changes per iteration)

Only executable predecessors feed a phi (`sccp_process_phis` consults
`SccpResult::executable_edges`), which is what makes the propagation
*conditional* rather than a plain constant fold.

**Registry builtin folds in the lattice** (issue #1134): when a caller
supplies `BuiltinFoldInputs` (`sccp_with_builtin_folds`), a
`Statement::AssignValue` whose RHS is a `[cmd args…]` command substitution is also
evaluated through the shared constant-substitution engine
(`rust/tcl-compiler/src/const_subst.rs`) — the registry `const_fold`
callbacks plus, when the caller proved a `TclOO` method frame, the
`[self class]` frame fact. The folded value re-enters the lattice at this
statement's use versions, so multi-statement chains
(`set base [self class]; set ns [namespace qualifiers $base]`) close under
SCCP's ordinary monotone fixpoint; nested substitutions carry a structural
depth cap. Only callers holding the whole-module command-mutation trust
fact pass the inputs — the shared per-unit lattice (and its salsa memo,
whose key cannot carry that fact) is built **without** them, and the
optimiser propagation pass re-runs SCCP with them when a body contains a
command-substitution assignment, overlaying the projection additively so
single-hop results stay byte-identical.

### Escaping names

A name that is not a private local of the frame is forced `OVERDEFINED`
regardless of what is assigned to it, so anything derived from it is
`OVERDEFINED` too. Three sources feed the escaping set:

- the per-function [`var_observability`](../../../rust/tcl-compiler/src/var_observability.rs)
  alias/trace lattice — `global`, `variable`, `my variable`, `upvar`,
  `namespace upvar`, and any name under a `trace`;
- whole-module facts the caller supplies (`extra_global_escaping`): for the
  **top-level** body, every name some other procedure declares `global`;
- for a **`TclOO` method** body, when the *propagation pass* asks for one:
  the class's instance variables — the class-level `variable` declarations
  plus the method's own (`MethodDef::instance_vars`). An instance variable is
  object state that outlives the method frame: the constructor or any other
  method may have written it, and a `my …` dispatch may rewrite it between two
  reads. This is the same escaping classification `elimination.rs` feeds
  through its cross-event channel to stop an instance-state write being
  deleted as a dead store; before issue #1097 the two passes disagreed, and
  `propagation.rs` had to keep variable propagation switched off wholesale for
  method bodies as a result.

  This one is a **propagation-only view**: `propagation.rs`
  (`oo_method_constants`) re-runs `sccp_with_extra_escaping` for the method
  rather than the unit's shared `FunctionUnit::sccp` being built that way,
  because other consumers legitimately read facts an instance variable
  carries — the object-collection element typing of issue #797 harvests
  `dict set pins $k [Pin new]` out of exactly such a name, and widening the
  shared lattice erases it. Re-running (rather than filtering the projected
  map) is what makes derived names (`set a $ivar ; set b $a`) drop out too.

What survives the projection for a method body is therefore a provably
method-local name, which no `my` / `next` / `[self …]` dispatch can reach.

#### The method-dispatch barrier and its evidence rules

A *proc* callee is modelled per call site: the CFG builder widens the caller's
defs at every call to a name in `cfg_builder::detect_upvar_procs`'s table. A
**method** reached through `my`, `next`, or an object dispatch is not in that
table and cannot be, because the dispatch never names its target. So the
barrier is answered from whole-module evidence, under one governing rule:
**when the evidence is incomplete, the barrier widens to abstention.**

Since issue #1164 the barrier is **per-method**
(`rust/tcl-compiler/src/optimiser/method_barrier.rs`), keyed by actual
reachability of the invalidating fact rather than a single module-wide
switch:

- A **bad** class is one defining a method — primary body, or any retained
  replacement body (`Module::redefined_methods`, issue #1166) — that can
  reach its caller's frame (`cfg_builder::upvar_info::reaches_caller_frame`),
  or one the lowering flagged unreadable (`Module::oo_unanalysed_classes`:
  a dynamic member name or member body).
- Classes are grouped into **hierarchy components**: the connected
  components of the `superclass` / `mixin` relations the lowering captures
  (`Module::class_relations`, recognised generically through the definer
  grammar's `MemberRefKind::Class`, with conservative tail matching when a
  relation is written bare). Within a component, `my` / `next` dispatch can
  land on any member class's method via the receiver's MRO; across
  unrelated components it cannot — under the same closed-world convention
  every other whole-module OO fact here uses.
- Each method's dispatch surface is classified from its statements: a
  registry head with the `TclOO` self-dispatch / next-chain traits targets
  the method's own component; a head naming a module class (`D new`)
  targets that class's component; a dynamic head (`$obj m`), an
  unresolvable literal head (a runtime object command, a `link`ed
  bareword), or a registry call handing a command prefix onward (`lsort
  -command …`) may dispatch anywhere; a call to a module proc inherits the
  proc's own dispatch surface transitively through the proc call graph.
  Reachability then closes over components.

A method is **barred** — its provably-local constants are not propagated —
iff a bad class is reachable from its dispatch surface (or a dispatch is
unbounded while any bad class exists). A method that never dispatches is
never barred. One caller-frame-reaching `classvar`-style helper therefore
disables propagation only for the classes that can actually reach it, not
for every method in the module. Module-wide widening remains for evidence
the lowering could not read at all: a dynamic OO definition target
(`OoDefinitionEvidence::dynamic_target`) or a dynamic `superclass` word
(`OoDefinitionEvidence::dynamic_class_relations`).

Three evidence sources were found incomplete in review, each a would-be
miscompile (the optimiser proposed folding `$x` to `1` where real Tcl prints
`2`, byte-identical on tclsh 9.0.4 and 8.6.14):

1. **Class state declared in another definition block.** `MethodDef::instance_vars`
   used to hold only the declarations of the block a method was extracted
   from, and `extract_oo_methods` builds a fresh set per block while keeping
   the first body of any method it has already seen. So
   `oo::class create C { method m {} {set x 1; my change; puts $x}; … }`
   followed by `oo::define C { variable x }` left `m` believing `x` was a
   private local. The lowering now accumulates a per-class union across every
   definition block (`Lowerer::class_instance_vars`) and merges it into every
   method of that class once all blocks are walked — order-free, since
   declaring `variable x` anywhere makes it instance state for every method of
   the class. This fixes `elimination.rs`'s dead-store protection at the same
   time, which reads the same field.

2. **A redefined method.** The lowering keeps the *first* body in
   `Module::methods` and, since issue #1166, retains every **replacement**
   body in `Module::redefined_methods`, so the caller-frame query scans
   them all. An initially-empty helper later redefined as
   `{upvar 1 x y; set y 2}` is caught by its retained replacement; a
   redefinition whose every body is caller-frame-clean no longer bars
   anything (the union of bodies over-approximates whichever is live at
   dispatch time). A replaced method in a *superclass* still bars a
   derived class's methods — the two classes share a hierarchy component.

3. **A caller-frame reach under a dynamic name.** The gate asks
   `cfg_builder::upvar_info::reaches_caller_frame`, the strictly structural
   query, *not* `var_observability`'s per-variable alias lattice. That route
   (`upvar_local_declaration_indices`) skips an `upvar` pair when either side
   starts with `$`, so `method helper {src} {upvar 1 $src b; set b 2}` — which
   mutates its caller's variable on every call — read as "no caller-frame
   alias". A dynamic name makes an alias *more* dangerous, never exempt.
   `reaches_caller_frame` counts every bucket `UpvarInfo::is_empty` covers
   plus `UpvarInfo::unnameable_local_aliases`, the set covering `upvar 1 x
   $dst`, whose alias the resolvable-buckets summary drops because it has no
   local name to file it under.

Rule 3's inverse matters too: `global` / `variable` / `namespace upvar` reach a
*namespace*, not the caller's locals, and must **not** trip the barrier — or
every ordinary class body would disable propagation.

Known evidence limits (pre-existing, shared by the old module-wide switch
and the per-method barrier): the lowering models `oo::class` /
`oo::define` *block* bodies only — an `oo::objdefine` per-object method,
or the single-member `oo::define C method m {…} {…}` spelling, contributes
no body to any of these scans, so a caller-frame reach hidden in one is
invisible to both gates.

### Constant branch detection

When a `Terminator::Branch` condition evaluates to a constant:

```rust
ConstantBranch {
    block: "entry_1".into(),
    span: Some(condition_span),
    condition: "$x".into(),
    value: true,
    taken_target: "if_then_3".into(),
    not_taken_target: "if_next_4".into(),
}
```

`block`, `taken_target`, and `not_taken_target` are block *names*
(`cfg::Function::block_name` of the corresponding `BlockId`) — the shape the
diagnostic aggregators need.  `span` points editors and CLIs at the
triggering site.

- The not-taken target's edge is never added to `executable_edges`, so the
  target is unreachable unless some other executable edge reaches it.
- O112 (constant condition elimination) is triggered.

### Existence-check folding (`info exists` / `array exists`)

`existence_constant_branches` (`rust/tcl-compiler/src/sccp.rs`) runs as a
**post-pass** over the CFG, not inside the SCCP fixpoint: the predicate is an
opaque `ExprNode::Command` and SCCP holds neither parameter nor existence
facts.  It contributes extra `ConstantBranch` entries for the
false-positive-free cases, feeding the analyser's `I230` and the optimiser's
`O101`.

The decision is **flow-insensitive**, taken against two whole-body scans
(`scan_defined_and_unset`: every name the body assigns, and every name a
literal `unset` names) plus the `ExistenceFrame` — the body's formal
`params` and, for a `TclOO` method, its `object_state`.  `existence_query_var`
recognises exactly `[info exists NAME]` / `[array exists NAME]`, optionally
under a `!`; anything embedded in a larger expression is declined.

- **parameter** — always bound, and bound as a *scalar*: `info exists` folds
  `true`, `array exists` folds `false` (issue #1239).
- **never assigned anywhere in the body** — folds `false` for either
  spelling.
- **assigned somewhere in the body** — no fold at all.  The scan is
  flow-insensitive, so "defined somewhere" does not prove "defined here".
- **element guard `X(elem)`** on an array the body never touches — folds
  `false` (issue #1173).  The decision is about the *array* name, so a
  dynamic key (`Params($k)`) folds just as well.

Abstentions, each declining the fold rather than guessing:

- any `Statement::Barrier` anywhere in the function disables the whole pass —
  an unknown command could `unset` or `upvar`-define the variable;
- a scope-alias local (`global` / `variable` / `upvar` / `namespace upvar` /
  a `trace` target, from `optimiser::elimination::scan_scope_aliases`) — its
  existence tracks the linked out-of-frame variable;
- a `TclOO` method's instance variables, unless a formal parameter of the
  same name shadows the declaration outright;
- a literal `unset` of a parameter, or `DynamicNameBarrier::destroys`, blocks
  the "parameter is present" fold; `DynamicNameBarrier::writes` blocks the
  "never defined, therefore absent" fold;
- a name that is not a bare `[A-Za-z0-9_]` local: a qualified name
  (`::ns::X`) may be populated outside the function's view.

### Unreachable blocks

Blocks that are never reached (due to constant branches, code after
`return`/`break`, etc.) are the complement of `SccpResult.executable_blocks`
— `FunctionUnit.sccp`, the return value of `sccp()`.  The optimiser derives
the set with `unreachable_blocks(&fu.cfg, &fu.sccp)`
(`rust/tcl-compiler/src/optimiser/elimination.rs`).  Taint analysis and
optimisation passes skip unreachable blocks.  (`FunctionAnalysis` has an
`unreachable_blocks` field, but nothing populates it — see the note above.)

### Type lattice

`FunctionUnit::types` maps each `ValueKey` to a `TypeLattice`
(`rust/tcl-compiler/src/types.rs`), whose `TypeKind` is the lattice rung:

```
Unknown  ──►  Known(shape)  ──►  Shimmered(shape set)  ──►  Overdefined
(bottom)     (exactly one)      (2..MAX_TYPE_UNION)         (top)
```

A lattice element carries a *bounded set* of `TypeShape`s, not a
from/to pair: `Known` is a one-element set, `Shimmered` a union of two or
more, and a union past `MAX_TYPE_UNION` collapses to `Overdefined`.  The
shimmer detector (S100–S102) reads the `Shimmered` rung.

| `TypeShape` | Meaning |
|---------|--------|
| `String` | String representation |
| `Int` | Integer that fits an `i64` |
| `Bignum` | Integer beyond a wide (`expr {2**64}`) |
| `Double` | IEEE-754 double |
| `Boolean` | Word booleans and 0/1 comparison results |
| `Numeric` | Abstract join of the numeric tower |
| `ByteArray` | Binary data |
| `List(Elements)` | Tcl list, with optional element facts |
| `Dict(Elements)` | Tcl dict, with optional facts about its values |
| `Object(Option<class>)` | `TclOO` / snit instance, class when known |
| `Channel` | I/O channel handle |

`TypeShape::coarse` projects a shape onto the registry's coarser `TclType`
vocabulary, which is what command specs are written against.

### Liveness analysis

`live_in[block]` / `live_out[block]` — the values that may still be read at
each block boundary.  There is no stored per-function liveness map: each
consumer computes what it needs from the `FunctionUnit`, via
`live_out_by_name()` (`rust/tcl-compiler/src/slot_allocation.rs`) for slot
interference and `liveness_dead_stores()`
(`rust/tcl-compiler/src/dead_stores.rs`) for dead stores.  The
`FunctionAnalysis.live_in` / `live_out` fields are declared and
unpopulated — see the note above.

`live_out_by_name` is keyed by variable **name**, not by `ValueKey`: slots
are per-name, so dropping SSA versions makes phi renaming across an edge a
no-op and the per-name result equals the name-collapse of a version-keyed
`live_out`.  It runs the standard backward fixpoint over the reverse of
`cfg::Function::reverse_postorder`, re-enqueuing only a block's predecessors
when its `live_in` changes.

A value is dead if it is defined but never appears in any `live_out` set.
Dead values trigger:
- O109 (dead store elimination) — variable set but never read
- O108 (aggressive DCE) — pure statement result never used

### Dead store detection

If `x₁ = "42"` and `x₁` never appears in any `uses` dict, it is a dead
store.  `liveness_dead_stores(fu, registry)`
(`rust/tcl-compiler/src/dead_stores.rs`) returns the `Vec<DeadStore>`
directly from the `FunctionUnit`; the diagnostics layer consumes it in
`emit_dead_store_diagnostics`
(`rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`).

### Read-before-set

If a variable is read at version 0 (never defined before use),
`emit_read_before_set_diagnostics`
(`rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`) reports it
straight off the `FunctionUnit`'s SSA and def-use facts → diagnostic
**W210**.  (`FunctionAnalysis.read_before_set` is a declared field with no
producer — see the note above.)

Existence checks are excluded: `info exists X` / `array exists X` test a
variable rather than reading its value, so the check reference itself is never
a read-before-set.  `existence_query_vars`
(`rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`) recognises both the
bare-call form (`info exists X`) and the command-substitution form
(`set y [info exists X]`, `puts [array exists X]`).

A check also narrows the region it dominates.  `collect_existence_guards`
(`rust/tcl-compiler/src/analyser/diagnostics/helpers.rs`) walks every
`Terminator::Branch` whose condition `expr_ast::existence_query_var`
recognises and emits a `(var, guard_block)` pair — the branch's true target
for a positive query, its false target for a `![info exists X]`.
`existence_exempt` then suppresses a read of that name in any block
`block_dominated_by` puts under the guard block, walking the SSA `idom`
chain.  The opposite branch keeps version 0, so a read there is still
flagged.  Narrowing is a runtime fact (the guard passed), so unlike the fold
it needs no foldability gate.

Only the exact three-word forms are recognised.  `existence_query_in_text`
splits the bracketed text on whitespace and requires exactly
`info exists NAME` or `array exists NAME`; the queried word is taken
verbatim, with no name-shape test of its own — `existence_constant_branches`
applies the bare-local shape gate itself, and the narrowing path applies
none.  Membership idioms (`[info vars X]` / `[info locals X]` compared with
`""`, `[llength [info vars X]]`, `[lsearch [info vars] X] > -1`) and
`catch {set _ $X}` are **not** recognised as existence proofs.

### Unused variables

Variables that are defined but never read (across all versions) are reported
by `emit_unused_variable_diagnostics`
(`rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`), again from the
`FunctionUnit` → diagnostic **W211**.  (`FunctionAnalysis.unused_variables`
is a declared field with no producer — see the note above.)

### Worked example — `set x 5; if {$x < 0} {…} elseif {$x > 0} {…} else {…}`

SCCP determines `x₁ = Const(Int(5))`:
- `5 < 0` → `false` → the `entry_1 → if_then_3` edge is not executable
- `5 > 0` → `true` → `if_then_5` taken, `if_next_6` (the `else` body) not executable
- `sign` resolves to `Const(Int(1))` — only one reachable definition reaches its phi

### Worked example — `set i 0; while {$i < 5} { incr i }`

- `i₁ = Const(Int(0))` (before loop)
- `i₂ = phi(i₁, i₃)` at `while_header_2` → `Overdefined` (loop-carried)
- `Statement::Incr` is not one of the shapes `evaluate_def` folds, so `i₃`
  is `Overdefined` and the phi cannot recover

## Decision rule

- If a value should be constant but is `Overdefined`, check whether a
  loop phi or barrier is widening it — or whether the defining statement is
  simply not a shape `evaluate_def` folds.
- `evaluate_def` folds `Statement::AssignConst`, `Statement::AssignExpr`
  through the expression evaluator, a `Statement::AssignValue` whose RHS is a
  literal / lattice-constant `$var` / foldable `[cmd …]`, and a
  single-variable single-list `foreach` (to the `ConstSet` of its elements).
  Every other statement kind — and every `Statement::Barrier` — widens its
  defs to `Overdefined`.
- Liveness is computed backward from uses to definitions — if a new IR
  node reads variables, ensure they appear in `SsaStatement::uses`.
- SCCP runs once per function (no iterative refinement across functions —
  that is interprocedural analysis), though the optimiser's propagation pass
  re-runs it with `sccp_with_builtin_folds` / `sccp_with_extra_escaping` when
  it needs a projection the shared per-unit lattice cannot carry.

## Related docs

- [Examples 3–7 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-3-expr-2--3)
- [GLOSSARY.md — SCCP, Lattice, Liveness](../../GLOSSARY.md#sccp)
- [kcs-cfg-ssa-fact-model.md](../../../docs/design/compiler/cfg-ssa-fact-model.md)
- [kcs-downstream-pass-contracts.md](../../../docs/design/compiler/downstream-pass-contracts.md)

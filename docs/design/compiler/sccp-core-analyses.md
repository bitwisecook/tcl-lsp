# KCS: SCCP and core analyses (Stage 6)

## Symptom

A contributor needs to understand how SCCP propagates constants, how
liveness analysis works, how the type lattice infers types, or why a
value is marked `OVERDEFINED` when it seems constant.

## Context

SCCP (Sparse Conditional Constant Propagation) runs over the SSA graph and
produces a `FunctionAnalysis` with constant values, type information,
liveness, dead stores, unreachable blocks, constant branches,
read-before-set, and unused variables.

Source: `rust/tcl-compiler/src/sccp.rs` (the propagation itself, plus
`sccp_with_extra_escaping` / `sccp_with_builtin_folds`),
`rust/tcl-compiler/src/analyses.rs` (`FunctionAnalysis`, `ConstantBranch`,
`DeadStore`, `ReadBeforeSet`, `UnusedVariable`, `LatticeValue`),
`rust/tcl-compiler/src/types.rs` (the type lattice)

## Content

### SCCP — constant propagation

The SCCP value lattice (`analyses::LatticeValue`, tagged by
`LatticeKind`):

```
Unknown  ──►  Const(v)  ──►  ConstSet({v1, v2, …})  ──►  Overdefined
(bottom)     (one known)     (a small set, up to        (top / varies
                              MAX_CONSTSET_SIZE = 32)     too much)
```

A `ConstSet` that grows past `MAX_CONSTSET_SIZE` widens automatically to
`Overdefined`.

SCCP walks the SSA graph and propagates:
- `Statement::AssignConst { value: "42", .. }` → `Const("42")`
- `Statement::AssignValue { value: "${x}", .. }` where `x₁ = Const("42")` → `Const("42")`
- Phi nodes: `phi(Const("42"), Const("42"))` → `Const("42")`
- Phi nodes: `phi(Const("42"), Const("99"))` → `ConstSet` of the two, widening to `Overdefined` past the cap
- Loop-carried values: `Overdefined` (value changes per iteration)

**Registry builtin folds in the lattice** (issue #1134): when a caller
supplies `BuiltinFoldInputs` (`sccp_with_builtin_folds`), an
`AssignValue` whose RHS is a `[cmd args…]` command substitution is also
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
   subclass's methods — the two classes share a hierarchy component.

3. **A caller-frame reach under a dynamic name.** The gate asks
   `cfg_builder::upvar_info::reaches_caller_frame`, the strictly structural
   query, *not* `var_observability`'s per-variable alias lattice. That route
   (`upvar_local_declaration_indices`) skips an `upvar` pair when either side
   starts with `$`, so `method helper {src} {upvar 1 $src b; set b 2}` — which
   mutates its caller's variable on every call — read as "no caller-frame
   alias". A dynamic name makes an alias *more* dangerous, never exempt.
   `reaches_caller_frame` counts every bucket of `UpvarInfo` plus
   `has_unnameable_local_alias`, the new flag covering `upvar 1 x $dst`, whose
   alias the resolvable-buckets summary drops because it has no local name to
   file it under.

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

When a `Terminator::Branch` condition evaluates to a constant, SCCP records
an `analyses::ConstantBranch`:

```rust
pub struct ConstantBranch {
    /// CFG block name containing the branch.
    pub block: String,
    /// Condition expression text.
    pub condition: String,
    /// Constant-evaluated condition result.
    pub value: bool,
    /// Target block when condition is true.
    pub taken_target: String,
    /// Target block when condition is false.
    pub not_taken_target: String,
}
```

- The not-taken target is marked unreachable.
- The constant-condition diagnostic (I230) and the optimiser's
  constant-branch fold / DCE (O101) both consume these entries.

### Existence-check folding (`info exists` / `array exists`)

`sccp::existence_constant_branches(cfg, frame, registry, …)` folds
`[info exists X]` / `[array exists X]` branch conditions into
`ConstantBranch` entries as a **post-pass** — SCCP itself cannot fold them,
because the predicate is an opaque `ExprNode::Command` and SCCP carries
neither parameter nor existence facts. The recogniser is
`tcl_syntax::expr::ast::existence_query_var`, which matches only the simple
two-word command-substitution form (optionally under `!`) and returns an
`ExistenceQuery { var, negated, command }`; the `ExistenceCommand::Info` and
`ExistenceCommand::Array` spellings are kept distinct because
`array exists` additionally asserts the binding is an array (issue #1239).

The frame's own facts arrive as an `ExistenceFrame { params, object_state }`
— built identically by the analyser (I230) and the optimiser (O101) so the
two cannot drift:

- **present** — a formal parameter is bound on entry, as a **scalar**:
  `info exists` folds to true, `array exists` to false. This holds for a
  defaulted parameter called with no argument, and a method parameter that
  shadows an instance-variable name still folds true.
- **absent** — a name this body never defines and never `unset`s folds to
  false. An element guard `X(elem)` on an array the body never touches folds
  false too (issue #1173) — the guard is decided on the array *base* name,
  so the element key may even be dynamic.
- `!` on the query flips the value.

Abstentions: only simple local names are folded, and only in functions free
of opaque barriers (an unknown command could `unset` or `upvar`-define the
variable). Scope-alias locals (`global` / `variable` / `upvar` /
`namespace upvar`) never fold, because their existence tracks the linked
out-of-frame variable. `ExistenceFrame::object_state` — a `TclOO` method's
`MethodDef::instance_vars` — likewise never folds either way: a class-level
`variable x` binds the name in every method frame without creating it, and
whether an earlier call on the same instance assigned it is not a
per-method fact. The dynamic-name barrier
(`rust/tcl-compiler/src/dynamic_names.rs`) gates the two directions
independently: a dynamic write (`set $switch {}`) can define any name and
kills the "absent" fold; a dynamic destroy (`unset $n`) can remove any name
and kills the "present" fold.

### Unreachable blocks

Blocks that are never reached (due to constant branches, code after
`return`/`break`, etc.) are collected in
`FunctionAnalysis::unreachable_blocks`. Taint analysis and optimisation
passes skip unreachable blocks.

### Type lattice

`types::TypeLattice` is a bounded, canonicalised **union of shapes** rather
than a fixed four-rung chain. Its coarse classification is `TypeKind`:

```
Unknown  ──►  Known (one shape)  ──►  Shimmered (2+ shapes)  ──►  Overdefined
```

`Known` is a singleton union and `Shimmered` is any multi-member union — two
*or more* differently-typed paths, not a from/to pair. The union is capped at
`MAX_TYPE_UNION` members; past that it widens to `Overdefined`.

Each member is a `TypeShape`:

| `TypeShape` | Meaning |
|-------------|---------|
| `String` | String rep |
| `Int` | Integer that fits an `i64` |
| `Bignum` | Integer beyond a wide (`expr {2**64}`) |
| `Double` | IEEE-754 double |
| `Boolean` | Word booleans and 0/1 comparison results |
| `Numeric` | Abstract join of the numeric tower |
| `ByteArray` | Binary data |
| `List(Elements)` | Tcl list, with optional element facts |
| `Dict(Elements)` | Tcl dict, with optional facts about its values |
| `Object(Option<Box<str>>)` | `TclOO` / snit instance, with its class when known |
| `Channel` | I/O channel handle |

`TypeShape::coarse()` projects a shape down to the registry vocabulary
(`TclType`), and `TypeShape::from_coarse()` lifts it back structure-free.
Purity (`typePtr == NULL`) is deliberately *not* a shape — whether an
intrep is committed is a program-point property tracked by the commit
dataflow (`rust/tcl-compiler/src/shimmer/commit.rs`), which is what the
shimmer detector (S100–S102) consumes.

### Liveness analysis

`FunctionAnalysis::live_in` / `live_out` are
`HashMap<String, HashSet<ValueKey>>` — the values that are "live" (may still
be read) at each block boundary.

A value is dead if it is defined but never appears in any `live_out` set.
Dead values trigger:
- O109 (dead store elimination) — variable set but never read
- O108 (aggressive DCE) — pure statement result never used

### Dead store detection

If `x₁ = "42"` and `x₁` is never used, it is a dead store, recorded in
`FunctionAnalysis::dead_stores` as `DeadStore { block, statement_index,
variable, version }`.

### Read-before-set

A variable read at version 0 (never defined before use) is recorded in
`FunctionAnalysis::read_before_set` as `ReadBeforeSet { block,
statement_index, variable }`, and the analyser reports it as **W210**.

Existence checks are excluded: `info exists X` / `array exists X` test a
variable rather than reading its value, so the check reference itself is
never a read-before-set. A check also narrows the region it dominates.
`analyser::diagnostics::helpers::collect_existence_guards(fu)` walks every
`Terminator::Branch`, recognises the condition with `existence_query_var`,
and returns `(var, guard_block)` pairs: a positive query guards the true
target, a negated one the false target. `existence_exempt` then suppresses a
read of `var` in any block dominated by `guard_block` (dominance walked over
`SsaFunction::idom` by `block_dominated_by`). The opposite branch keeps
version 0, so a read there is still flagged.

Narrowing is a runtime fact — the guard passed — so unlike the constant fold
above it needs no foldability gate. It is also deliberately narrower than the
fold: only the exact `[info exists X]` / `[array exists X]` command
substitution is recognised. Membership idioms (`info vars` / `info locals`
comparisons, `lsearch` over `info vars`, `catch {set _ $X}`) are **not**
recognised as existence evidence.

Several other shapes are exempted alongside the guards, in the same
read-before-set walk (`analyser/diagnostics/dataflow.rs`): a barrier body
that locally sets the name, a read-modify-write call that auto-creates its
target (`lappend` / `append` — but not `unset`, whose missing-variable case
is W213), and a use site that itself safely initialises the variable
(`safe_on_uninit` commands, or an `incr` of its own target).

### Unused variables

Variables that are defined but never read (across all versions) appear in
`FunctionAnalysis::unused_variables` as `UnusedVariable { block,
statement_index, variable }` → diagnostic W211. Unused formal parameters are
tracked separately in `FunctionAnalysis::unused_params`.

### Worked example — `set x 5; if {$x < 0} {…} elseif {$x > 0} {…} else {…}`

SCCP determines `x₁ = Const("5")`:
- `5 < 0` → `Const(false)` → `if_then_3` unreachable
- `5 > 0` → `Const(true)` → `if_then_5` taken, `if_next_6` unreachable
- `sign` resolves to `Const("1")` (only one reachable definition)

### Worked example — `while {$i < 5} { incr i }`

- `i₁ = Const("0")` (before loop)
- `i₂ = phi(i₁, i₃)` at loop header → `Overdefined` (loop-carried)
- SCCP cannot fold loop induction variables

## Decision rule

- If a value should be constant but is `Overdefined`, check whether a
  loop phi or barrier is widening it.
- Pure commands can be inferred through without invalidating the lattice.
  Impure commands force all potentially affected values to `Overdefined`.
- Liveness is computed backward from uses to definitions — if a new IR
  node reads variables, ensure they appear in the SSA statement's `uses`.
- SCCP runs once per function (no iterative refinement across functions —
  that is interprocedural analysis).

## Related docs

- [Examples 3–7 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-3-expr-2--3)
- [GLOSSARY.md — SCCP, Lattice, Liveness](../../GLOSSARY.md#sccp)
- [kcs-cfg-ssa-fact-model.md](../../../docs/design/compiler/cfg-ssa-fact-model.md)
- [kcs-downstream-pass-contracts.md](../../../docs/design/compiler/downstream-pass-contracts.md)

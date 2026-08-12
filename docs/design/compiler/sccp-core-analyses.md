# KCS: SCCP and core analyses (Stage 6)

## Symptom

A contributor needs to understand how SCCP propagates constants, how
liveness analysis works, how the type lattice infers types, or why a
value is marked `OVERDEFINED` when it seems constant.

## Context

`analyse_function()` in `analyses.rs` runs SCCP (Sparse Conditional
Constant Propagation) over the SSA graph, producing a `FunctionAnalysis`
with constant values, type information, liveness, dead stores, unreachable
blocks, constant branches, read-before-set, and unused variables.

Source: `rust/tcl-compiler/src/analyses.rs` / `rust/tcl-compiler/src/sccp.rs` (`analyse_function` at line 3964, `FunctionAnalysis` at line 426),
`rust/tcl-compiler/src/types.rs`

## Content

### SCCP — constant propagation

The SCCP value lattice:

```
UNKNOWN  ──►  CONST(value)  ──►  OVERDEFINED
 (bottom)      (provably constant)    (top / multiple possible values)
```

SCCP walks the SSA graph and propagates:
- `IRAssignConst(value="42")` → `CONST("42")`
- `IRAssignValue(value="${x}")` where `x₁ = CONST("42")` → `CONST("42")`
- Phi nodes: `phi(CONST("42"), CONST("42"))` → `CONST("42")`
- Phi nodes: `phi(CONST("42"), CONST("99"))` → `OVERDEFINED`
- Loop-carried values: always `OVERDEFINED` (value changes per iteration)

**Registry builtin folds in the lattice** (issue #1134): when a caller
supplies `BuiltinFoldInputs` (`sccp_with_builtin_folds`), an
`IRAssignValue` whose RHS is a `[cmd args…]` command substitution is also
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

When a `CFGBranch` condition evaluates to a constant:

```python
ConstantBranch(
    block="entry_1",
    condition="$x",
    value=True,
    taken_target="if_then_3",
    not_taken_target="if_next_4",
)
```

- The not-taken target is marked unreachable.
- O112 (constant condition elimination) is triggered.

### Existence-check folding (`info exists` / `array exists`)

`_ExistenceFolder` rewrites `[info exists X]` / `[array exists X]`
sub-expressions in a branch condition to a `0`/`1` literal before the branch
decision is taken, so an existence guard becomes an ordinary constant branch
(feeding `I230` + DCE):

- **absent** — a use at SSA version 0 of a plain local scalar that is not a
  parameter and not frame-escaping (`global`/`variable`/`upvar`) → fold to
  `0`. (`array exists` also folds `0`.)
- **present** — a use whose reaching version is a real assignment, or a
  parameter → `info exists` folds to `1` (a scalar `set` does not prove
  `array exists`, so that stays unfolded).
- **`unset`** version → fold to `0`.

The fold is gated per function: it is disabled whenever any statement could
create or destroy a local invisibly (any barrier/`IRBlock`/`IRUpFrame`, an
`IRCall` with an UNKNOWN-target write, a dynamic barrier, or an inline `BODY`
argument). Array elements (`A(k)`) and qualified names (`::ns::X`) are never
folded. This keeps the rewrite sound for DCE/codegen.

### Unreachable blocks

Blocks that are never reached (due to constant branches, code after
`return`/`break`, etc.) are collected in `FunctionAnalysis.unreachable_blocks`.
Taint analysis and optimisation passes skip unreachable blocks.

### Type lattice

```
UNKNOWN  ──►  KNOWN(TclType)  ──►  SHIMMERED(from, to)  ──►  OVERDEFINED
```

| TclType | Values |
|---------|--------|
| `INT` | `"42"`, `"0xFF"` |
| `DOUBLE` | `"3.14"` |
| `BOOLEAN` | `"true"`, `"false"`, `"1"`, `"0"` |
| `STRING` | Any non-numeric text |
| `LIST` | Tcl list format |
| `DICT` | Tcl dict format |
| `NUMERIC` | Abstract join of INT and DOUBLE |

`SHIMMERED(from_type, to_type)` tracks forced type conversions — used by
the shimmer detector (S100–S102).

### Liveness analysis

`live_in[block]` / `live_out[block]` — sets of `SSAValueKey` that are
"live" (may still be read) at each block boundary.

A value is dead if it is defined but never appears in any `live_out` set.
Dead values trigger:
- O109 (dead store elimination) — variable set but never read
- O108 (aggressive DCE) — pure statement result never used

### Dead store detection

If `x₁ = "42"` and `x₁` never appears in any `uses` dict, it is a dead
store.  SCCP marks it in `FunctionAnalysis.dead_stores`.

### Read-before-set

If a variable is read at version 0 (never defined before use), it appears
in `FunctionAnalysis.read_before_set` → diagnostic W103.

Existence checks are excluded: `info exists X` / `array exists X` test a
variable rather than reading its value, so the check reference itself is never
a read-before-set. A check also narrows the region it dominates —
`_existence_narrowed_blocks` records, for each block, the names a dominating
guard proves to exist (the true region of `if {[info exists X]}`, or the false
region of a negated guard), and reads of those names there are suppressed. The
opposite branch keeps version 0, so a read there is still flagged.

`_existence_implications` recognises the membership idioms too, all restricted
to a single exact (non-glob) name: `[info vars X]` / `[info locals X]`
compared with `""` (`ne`/`eq`), `[llength [info vars X]]`, and
`[lsearch [info vars] X] > -1` / `>= 0` / `!= -1`. `info globals` is *not*
narrowed (it proves the global exists, not the bare-`$X` local), and unsound
`lsearch` options (`-regexp`, `-nocase`, …) are rejected. `catch {set _ $X}`
is recognised too: a zero (no-error) result proves the read succeeded, so the
*false* branch knows `X` exists — the true (error) branch is ambiguous (missing
*or* an array read as a scalar) and proves nothing. Narrowing is a runtime fact
(the guard passed), so unlike the fold it needs no foldability gate.

The exact-name shape test goes through `shared.naming.is_unqualified_var_name`
(built on the lexer's `is_bare_var_name`), not a local regex.

### Unused variables

Variables that are defined but never read (across all versions) appear in
`FunctionAnalysis.unused_variables` → diagnostic W104.

### Worked example — `set x 5; if {$x < 0} {…} elseif {$x > 0} {…} else {…}`

SCCP determines `x₁ = CONST("5")`:
- `5 < 0` → `CONST(false)` → `if_then_3` unreachable
- `5 > 0` → `CONST(true)` → `if_then_5` taken, `if_next_6` unreachable
- `sign` resolves to `CONST("1")` (only one reachable definition)

### Worked example — `while {$i < 5} { incr i }`

- `i₁ = CONST("0")` (before loop)
- `i₂ = phi(i₁, i₃)` at loop header → `OVERDEFINED` (loop-carried)
- SCCP cannot fold loop induction variables

## Decision rule

- If a value should be constant but is `OVERDEFINED`, check whether a
  loop phi or barrier is widening it.
- Pure commands can be inferred through without invalidating the lattice.
  Impure commands force all potentially affected values to `OVERDEFINED`.
- Liveness is computed backward from uses to definitions — if a new IR
  node reads variables, ensure they appear in `SSAStatement.uses`.
- SCCP runs once per function (no iterative refinement across functions —
  that is interprocedural analysis).

## Related docs

- [Examples 3–7 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-3-expr-2--3)
- [GLOSSARY.md — SCCP, Lattice, Liveness](../../GLOSSARY.md#sccp)
- [kcs-cfg-ssa-fact-model.md](../../../docs/design/compiler/cfg-ssa-fact-model.md)
- [kcs-downstream-pass-contracts.md](../../../docs/design/compiler/downstream-pass-contracts.md)

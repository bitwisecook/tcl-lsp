# SCCP and core analyses (Stage 6)

How sparse conditional constant propagation, liveness, and the type lattice
work together over the SSA graph, and why a value that looks constant can
still settle at `OVERDEFINED`.

`sccp()` in `sccp.rs` runs SCCP (Sparse Conditional Constant Propagation) over
the SSA graph, producing an `SccpResult` with the per-value lattice, the
executable blocks and edges, and the constant branches.  Type information,
liveness, dead stores, read-before-set, and unused variables are produced by
separate passes and reach consumers through the per-function `FunctionUnit`
or a pass-local return value such as `liveness_dead_stores()`'s
`Vec<DeadStore>`.

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
- Loop-carried values: `Overdefined` (value changes per iteration); after a
  loop the solver runs to its exit, the value it leaves (§ *Bounded loops*)

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
`OVERDEFINED` too. Four sources feed the escaping set:

- the per-function [`var_observability`](../../../rust/tcl-compiler/src/var_observability.rs)
  alias/trace lattice — `global`, `variable`, `my variable`, `upvar`,
  `namespace upvar`, and any name under a `trace`;
- whole-module facts the caller supplies (`extra_global_escaping`): for the
  **top-level** body, every name some other procedure declares `global`;
- the names the module's callback scripts write, destroy or bind
  (`Module::deferred_writes`, carried by `TraceInputs` into every function,
  the top level and each procedure): an `after`, `fileevent`, `bind` or
  variable-trace script runs outside the code that registered it, and so do a
  procedure named as one and a lambda one applies, whose global writes are
  read as a procedure summary reads a body. The registry states which words
  are such scripts
  (`CommandRegistry::callback_script_indices`); the scan over them
  (`deferred_writes.rs`) names no command. A callback that writes a computed
  name makes every name escaping, as a trace on a computed name does, and so
  does a callback the scan cannot read: a word computed at run time, a
  substitution of a command that builds no command prefix, a `{*}` expansion,
  a computed command head, a command that is neither a procedure of the module
  nor one of the registry. A script spelled as several words (`after 100 set
  done 1`) is read as the one script it concatenates into;
- for a **`TclOO` method** body, when the *propagation pass* asks for one:
  the class's instance variables — the class-level `variable` declarations
  plus the method's own (`MethodDef::instance_vars`). An instance variable is
  object state that outlives the method frame: the constructor or any other
  method may have written it, and a `my …` dispatch may rewrite it between two
  reads. This is the same escaping classification `elimination.rs` uses to
  stop an instance-state write being deleted as a dead store.

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

The escaping set is name-based and whole-function. A call to a command the
module cannot see, with a computed command, or inside the body of an opaque
`catch` adds a flow-sensitive fact beside it: the CFG marks the call
(`SyntheticMarker::UnseenCall`), the SSA gives each name live past the marker
a fresh version (`SsaFunction::value_clobbers`), which the solver states
`OVERDEFINED` — save a parameter seed of a procedure a caller proved pure — and
records the version each name held there
(`SsaFunction::is_observed_by_unseen_call`), which the code may read. So a
plain top-level name after such a call is as undecided as its `::` spelling,
and a procedure's local too, which the callee reaches through `upvar 1`; a use
before the call keeps its value, and a definition made afterwards is decided
again.

#### The method-dispatch barrier and its evidence rules

A *proc* callee is modelled per call site: the CFG builder widens the caller's
defs at every call to a name in `cfg_builder::detect_upvar_procs`'s table. A
**method** reached through `my`, `next`, or an object dispatch is not in that
table and cannot be, because the dispatch never names its target. So the
barrier is answered from whole-module evidence, under one governing rule:
**when the evidence is incomplete, the barrier widens to abstention.**

The barrier is **per-method**
(`rust/tcl-compiler/src/optimiser/method_barrier.rs`), keyed by actual
reachability of the invalidating fact:

- A **bad** class is one defining a method — primary body, or any retained
  replacement body (`Module::redefined_methods`) — that can reach its
  caller's frame (`cfg_builder::upvar_info::reaches_caller_frame`), or one
  the lowering flagged unreadable (`Module::oo_unanalysed_classes`: a dynamic
  member name or member body).
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

Three evidence rules keep the barrier sound; each guards a would-be
miscompile (folding `$x` to `1` where tclsh 9.0.4 and 8.6.14 print `2`):

1. **Class state is unioned across definition blocks.** The lowering
   accumulates a per-class union of `variable` declarations across every
   `oo::class create` / `oo::define` block (`Lowerer::class_instance_vars`)
   and merges it into every method of that class once all blocks are walked
   (`MethodDef::instance_vars`), so
   `oo::class create C { method m {} {set x 1; my change; puts $x} }`
   followed by `oo::define C { variable x }` still sees `x` as instance
   state in `m`. `elimination.rs`'s dead-store protection reads the same
   field.

2. **Replacement bodies are scanned.** The lowering keeps the *first* body
   in `Module::methods` and retains every **replacement** body in
   `Module::redefined_methods`, so the caller-frame query scans them all. An
   initially-empty helper later redefined as `{upvar 1 x y; set y 2}` is
   caught by its retained replacement; a redefinition whose every body is
   caller-frame-clean bars nothing (the union of bodies over-approximates
   whichever is live at dispatch time). A replaced method in a *superclass*
   still bars a derived class's methods — the two classes share a hierarchy
   component.

3. **A dynamic `upvar` name is a caller-frame reach.** The gate asks
   `cfg_builder::upvar_info::reaches_caller_frame`, the strictly structural
   query, *not* `var_observability`'s per-variable alias lattice, which
   (`upvar_local_declaration_indices`) skips an `upvar` pair when either
   side starts with `$` and would read
   `method helper {src} {upvar 1 $src b; set b 2}` as "no caller-frame
   alias". A dynamic name makes an alias *more* dangerous, never exempt.
   `reaches_caller_frame` counts every bucket `UpvarInfo::is_empty` covers
   plus `UpvarInfo::unnameable_local_aliases`, the set covering `upvar 1 x
   $dst`, whose alias the resolvable-buckets summary drops because it has no
   local name to file it under.

Rule 3's inverse matters too: `global` / `variable` / `namespace upvar` reach a
*namespace*, not the caller's locals, and must **not** trip the barrier — or
every ordinary class body would disable propagation.

Known evidence limits: the lowering models `oo::class` / `oo::define`
*block* bodies only — an `oo::objdefine` per-object method, or the
single-member `oo::define C method m {…} {…}` spelling, contributes no body
to any of these scans, so a caller-frame reach hidden in one is invisible to
both gates.

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
    kind: BranchFactKind::Applied,
}
```

`block`, `taken_target`, and `not_taken_target` are block *names*
(`cfg::Function::block_name` of the corresponding `BlockId`) — the shape the
diagnostic aggregators need.  `span` points editors and CLIs at the
triggering site.

- The not-taken target's edge is never added to `executable_edges`, so the
  target is unreachable unless some other executable edge reaches it.
- O112 (constant condition elimination) is triggered.
- A `while` or `for` header is one of these branches: W240 and W241 read its
  fact at the condition word's span (§ *Loop headers*).

Every `ConstantBranch` carries a `kind: BranchFactKind`. `Applied` is a
branch the SCCP fixpoint itself decided — an `if`, `while` or `for`
condition, an exact `switch`'s dispatch chain, an existence query.
`Proven` is a condition proven without updating reachability; no producer
states it today. `Selected` is an arm of an opaque `switch` that no member
of the subject runs the body of (§ *Selection records*): it has no block of
its own, so `taken_target` and `not_taken_target` are empty, `value` is
`false`, `span` is the arm's pattern, and nothing is applied to
`executable_blocks`. An emitter reads only the kind it owns —
`emit_constant_branch_diagnostics` the `Applied` facts,
`emit_selected_arm_diagnostics` the `Selected` ones — and never re-derives
the proof from a frame of its own, so `compiler_checks.rs` reports whichever
kind is stored without asking which pass produced it. A consumer that reads
every stored fact by its block reads the kind too: the O100 hint in
`compiler_checks.rs` is for a branch only, and branch folding skips a
`Selected` fact, whose block also ends in the next `if`'s branch.

### Loop headers

A conditional loop's header is an ordinary branch. The analyser's walk queues
each loop it examines with what the loop's text says, and the per-function
pass resolves the queue against the unit's `Applied` fact at the loop's
condition span (`HeaderFact`): a header false at entry is W240, and one true
at every test is W241 when no executable path leaves the loop — the loop's
end block is executable, which a `break` makes so, or a path from the body
reaches a `return` — and the body's text holds no exit command either, which
covers the `break` inside a `catch` body the CFG does not lower. Either
verdict replaces W242, the hint that a counter is never modified. A loop no
unit decides — its header is not reached, its condition varies, a
stub-declared loop command the CFG keeps as a call, a complexity-guarded
body — keeps the verdict its text gives, reported after the pass.

What the text gives includes the loop's counter, as its iteration plan states
it (`LoopCounter`): the condition compares one variable with a literal bound,
and every pass adds one literal step — the counted plan's step script, or the
one increment at the top level of a conditional plan's body, each the
registry's integer cell update run over its words — with nothing else in the
loop writing the variable and nothing in the body leaving it, however the
word that holds the exit or the write is written. A counter that never
reaches its bound from the integer a `for`'s start script writes is W241 by
the text alone. A loop the unit holds and does not decide reads the integer
the solver proves the counter holds at the block its passes start from
instead (`LoopTerminationCandidate::seed`), so `set i $start` before the loop
is checked as `set i 5` is, a `while` among them:

```text
set start 5; set i $start
while {$i < 10} {incr i -1}    ;# W241: counter $i starts at 5, moves by -1 per step
```

I230 reads a loop's own test structurally: the branch whose false edge enters
the block the loop leaves to (`cfg.loop_nodes`) is the loop's test, and a
decided true one — the idiomatic `while 1` — is not reported. A decided `if`
in the loop's body, or right after the loop in the block it leaves to, is an
`if` like any other, whatever its block's name.

### Bounded loops (`SccpResult::loop_enumerations`)

The solver runs a loop to its exit where it proves the state the loop starts
from ([value-transfers.md](value-transfers.md) § *Bounded-loop
enumeration*). The CFG builder records each `for`, `while` and `foreach` it
lowers as a `LoopNode`, keyed by the block the loop leaves to and naming the
block whose exit state its passes start from — the end of a `for`'s start
script, the block before the loop otherwise. After the fixed point,
`enumerate_loops` takes each loop whose blocks are executable, builds its
start state (`start_state`: each value the settled lattice proves exact at
that block's exit, under the refinements in force there, a scalar of unknown
value or no binding where the existence rung proves it so) and runs
`static_loops::enumerate_loop` over it. A statement's word is read over the
state: a literal word as its value, a variable as the state holds it, and a
`[…]` script as the one command it names, run by its route under the
effect-free policy, so `incr x [expr {$b / $a}]` adds the quotient the
state's `a` and `b` give, and a script that stores declines the loop. A loop that leaves normally — its
iterable exhausted, its condition false, or `break` — states each place it
wrote as an exact-value `EdgeRefinement` on every executable edge into the
block it leaves to, for the version live there, and the solver runs again
with those among the refinements it reads (`RefinementFlow`, where one fact
stated on several edges is one fact, so a block every one of those edges
enters holds it). The second run enumerates nothing, and its values only
descend from the first's. Inside the loop nothing changes: the header φs
widen as before, so a body statement still reads `ConstSet` or
`Overdefined`.

```text
for {set i 0} {$i < 5} {incr i} {}
if {$i == 5} {puts five}       ;# I230: always true; O101 folds the branch
```

A loop the enumeration declines — a read of a place the state holds no
value of, a store to a place it does not hold or another actor may write, a
statement no route evaluates exactly, more than
`DEFAULT_MAX_STATIC_LOOP_ITERS` passes (`Budget(Iterations)`) — or one that
leaves with an error or a `return`, publishes nothing, and the branch after
it decides only as the widened lattice lets it. Each loop run is recorded
as an `EnumeratedLoop { exit_block, span, iterations, exit, published }` in
`SccpResult::loop_enumerations`, in the order of the blocks the loops leave
to (`rebase_function_unit`-shifted), and the Explorer's `sccp` view prints
it as `enumerated loop: 5 iterations, false condition`, with the block it
leaves to and each value it published. The ranges read the state too: an
integer a loop leaves is the version's interval where the state is in force
(`intervals::refine_interval`), past the loop header's widening; and the
argument-sensitive O103 re-run, which solves the callee under the call's
seeds, reads each return's value at its block (`SccpResult::value_at`).

An opaque `catch`'s body is run the same way at the marker that states its
writes (`catch_body_answer`): over the exact state of the names the marker
and its call read, the body runs to its end — the `catch` absorbs however it
completes — and each name it wrote takes what it left, a value or no
binding, while each name it did not write keeps what it held. A body the
enumeration declines, or one that writes a name the marker does not state,
leaves the marker to its may-definitions as before.

### Selection records (`SccpResult::selections`)

A `switch` the CFG builder does not flatten — `-glob`, `-regexp`, `-nocase`,
a fall-through arm, and `case` — stays one `Statement::Switch` in one block,
and the solver states its decision beside the block rather than in it. After
the fixed point, `LatticeDriver::selection_facts` (`value_transfer.rs`)
visits each executable opaque `Statement::Switch` and asks the command its
binding site names (`cfg.command_binding_sites`, trusted by the module) for
its `Selection` transfer over the settled lattice: a `SelectionFact {
selected, bodies, writes }` with one entry per member of an exact or finite
subject — the arm each member's pattern selects, the arm whose body runs (the
two differ across a `-` fall-through), and the capture writes. The answer is
recorded only where the command's structural plan reads the statement's own
subject and clause count, and a member the transfer declines — a pattern it cannot decide, a
`-nocase` a profile spanning 8.4 may not have, a quoted or braced `-` body
under a profile that may be 9.1, whose compiled and interpreted paths read
the word two ways — declines the whole fact and records nothing.

```rust
SelectionRecord {
    span: statement_span,
    arm_pattern_spans: vec![…],          // one per arm the statement keeps
    fact: SelectionFact { selected, bodies, writes },
}
```

An arm index counts the command's pattern and body pairs, and the index one
past `arm_pattern_spans` is the final `default` the statement keeps as its
default body (`SelectionRecord::is_default`). Beside each record the pass
states a `Selected` branch fact for every arm no member runs the body of. An
arm that passes its body on with `-` is judged by the body it leads to, so
the alternates of a running body are not reported, and the final `default`,
which has no pattern span, never is.

The exact, case-sensitive form without a fall-through arm needs no record:
the CFG builder flattens it into a dispatch chain of `StrEq` branches, and
`evaluate_branch` reads a whole-variable subject from the lattice when the
name owner proves the operand is exactly one variable reference
(`whole_variable_operand`), so each arm is an `Applied` branch and the dead
arms' blocks leave `executable_blocks`.

The record states a selection, not the writes: the SSA gives an opaque
`switch` a may-definition of every name its arms write or bind
(`ssa::switch_may_defs`), so the solver takes the value a name holds after it
as the join of the value before and `OVERDEFINED`, whether or not a record
proves which arm runs. What a command an arm runs does to the frame follows
the statement: the names a callee writes through `upvar` are may-definitions
of a marker after it, and a command that may write any name adds the
caller-frame barrier ([value-transfers.md](value-transfers.md) § `switch`).

Consumers read the one decision. O112 folds a `switch` only from it: the
record at the statement's span for an opaque form, the `Applied` facts of the
chain for a flattened one. The analyser's `switch_body_is_selected` and the
loop enumeration's `exec_switch`, which hold words rather than a lattice, ask
the same `Selection` transfer through `value_transfer::literal_selection` and
`statement_selection`. I231 reports each `Selected` arm. A selection never
applies reachability, because no block stands for an arm: O107 does not fire
for an opaque form, and no edit deletes an arm.

### Existence-check folding (`info exists` / `array exists`)

An existence query decides inside the fixed point.
`[info exists NAME]` and `[array exists NAME]` — recognised by the
resolved operation (`IntrinsicId::InfoExists` / `ArrayExists`, through
`existence_query::kind_of`), never by spelling — answer through the
driver's nested service as a read of the existence rung below at the
point the condition or word is evaluated: a `Bound(_)` place exists for
`info exists`, a `Bound(Array)` one is an array for `array exists` and a
`Bound(Scalar)` one is not, and an `Unbound` place is neither. An element
query reads its array: an unbound array holds no element, and any other
fact decides nothing about one. A computed key leaves a bareword array
fixed, so `[info exists Params($k)]` reads `Params` too
(`existence_query::computed_element_base`). `MayBound`, `Bound(Either)`
for `array exists`, a run without the rung (`Unavailable`, never read as
unbound), any other computed name, and a registry special variable in the
initial global frame — which the host binds, not the script — decide
nothing. A name the
function only asks about gets a rung slot of its own past the SSA's
symbols, so a clobber reaches it like any other place.

The decided condition is an ordinary `Applied` branch: it updates
`executable_blocks`, so O107, taint, shimmer and the reachability-gated
checks see the dead arm, and the analyser's I230 and the optimiser's O101
read the one fact. The abstentions the post-pass it replaces kept by hand
— a barrier or `UpFrame`, a scope alias, an instance variable, a
connection-scoped iRules name, a computed write or destroy — are the
rung's entry rules and clobbers.

### The existence rung (`SccpResult::existence`)

Beside the values, the run carries a flow-sensitive bound / unbound fact per
place ([value-transfers.md](value-transfers.md) § *Existence*): the
registry's `Existence` (`Pending` ⊥, `Unbound`, `Bound(Scalar | Array |
Either)`, `MayBound` ⊤), joined as that page states. It is a forward fact
per place over the same executable blocks and edges the value lattice runs
over, asked for by the caller through `TraceInputs::existence`
(`ExistenceEntry`); a run that does not ask for it — the optimiser's
re-runs, a detached evaluation — has none, and every existence read in it
is `Unavailable`, never `Unbound`.

- **Entry.** Parameters enter `Bound(Scalar)`; an externally mutable place
  (qualified, in the escaping set — scope aliases and traced names included
  — or under a computed trace) enters `MayBound` and every definition of it
  leaves it `MayBound`, and no guard refines it; a `TclOO`
  method's instance variables and an iRules `when` handler's
  connection-scoped names (every name a handler of the module binds,
  `AnalysisContextKey::connection_scoped`) enter `MayBound`; in the initial
  global frame a registry special variable (a pack-declared one included)
  enters bound as its kind when startup binds it, `MayBound` otherwise;
  every other local enters `Unbound`. The CFG builder's caller-frame barrier
  applies from the entry.
- **Transfer.** A block enters with the join of its executable edges' exits
  — across a `try` / `catch` exception edge, of every point in the region
  the handler covers — which is each φ's fact. A typed assignment binds its
  place (an element write binds the element as a scalar and its array as an
  array); a call takes the steps its evaluated outcome's stores state
  (`Write` binds, `Preserve` keeps, `Unbind` unbinds, `MayWrite` joins with
  bound), else the declaration's own existence transfer on the normal
  completion, else the generic transfer, which widens to `MayBound`; the
  synthetic loop header binds its binders once the list is proven to have an
  element and leaves them as they were over a proven-empty list.
- **Clobbers.** A `Barrier` or `UpFrame` makes every place `MayBound`, as
  it widens every value, and so does a call to a command the module cannot
  see (`SyntheticMarker::UnseenCall`); a computed name is
  applied from its own statement
  on (`dynamic_names::statement_barrier`): a dynamic write turns an
  `Unbound` place `MayBound`, a dynamic destroy a bound one; a statement that
  keeps a nested body inline makes every place the body defines or unsets
  `MayBound`, and so does a script body a command substitution runs in
  this frame (`[catch {unset x}]`, `[eval {…}]`, `[lmap v {1} {…}]`),
  in a statement's words or in a branch condition or returned word —
  every place, when such a body holds a barrier or an up-frame.
- **Edge refinement.** A branch whose condition states an
  existence fact refines the place on that edge (`EdgeRefinement`, the
  design's shape with its domain `FactDomain::Existence`): the true edge of
  `[info exists x]` carries `Bound(Either)` and its false edge `Unbound`;
  the true edge of `[array exists x]` carries `Bound(Array)` and its false
  edge nothing; `!` swaps the edges, `C1 && C2` states both true-edge
  answers on its true edge and `C1 || C2` both false-edge answers on its
  false edge. `[info exists a(k)]` binds the element as a scalar and `a` as
  an array on its true edge and unbinds the element on its false edge; a
  computed key binds only a bareword array, on the true edge. The fact
  narrows the place as the edge arrives — a `MayBound` place to it, a
  `Bound(Either)` one to a kind — and a fact the place contradicts, on an
  edge the query did not decide (a special variable the host binds),
  leaves it as it is. An externally mutable place is never refined:
  a qualified, aliased, traced or computed-trace place, a `TclOO`
  instance variable, a cross-event iRules name and a special variable of
  the initial global frame can be set or unset by a plain call to a
  procedure the module cannot see, with no barrier in between, so an
  existence query about one decides nothing either (`EscapingPlace`). A
  place of the procedure's own frame is refined, and its refinement ends
  at the next barrier or up-frame, where every place is `MayBound` again.
  The refined fact flows on through every block the arm reaches, joining
  at a merge like any other.
- **Answer.** `SccpResult::existence` per SSA version (version 0 its entry
  fact), `existence_reads` per statement and place it reads — after any
  clobber since the version's definition and any refinement since the
  guard — and `existence_exits` per block, what its terminator reads.
  `SccpResult::existence_at(block, key)` is the block-qualified lookup: the
  fact the version live at the block's entry holds there — from
  `existence_entries` where a refinement or a clobber changed it on the
  way in, else the version's own. `SccpResult::refinements` lists the
  function's refinements in every domain — the rung's and those
  § *Edge refinements* records — one per edge, place and fact.

The cell updates read it through `prior_store(place, FactDomain::Existence)`:
an `Unbound` place is an absent cell, which `append` and `lappend` create in
every release and `incr` from 8.5 (`creates_absent`), so `incr fresh`
evaluates to 1 under `tcl8.5` onwards and declines with `UnboundPlace` under
`tcl8.4` and a profile spanning both; the `dict` keyed updates create an
absent dictionary the same way.

Three writers read it the same way. `const` (from 9.0)
writes its value only into an `Unbound` place: over an existing variable
it raises and over an existing constant it keeps the old value, so any
other place declines, and the place is bound as a scalar after it either
way. `array unset` without a pattern unbinds an array but leaves a scalar
or an absent name alone — it never raises — so it reads the prior fact,
and a place that may be either widens; with a pattern the array stays.
`array default` (from 9.0) may bind its name as an array.

### Edge refinements (`SccpResult::refinements`)

A branch condition's `Selection` transfer
(`tcl_expr_eval::condition_edge_facts`) states, per edge, what the
condition's outcome proves about the places it reads
([value-transfers.md](value-transfers.md) § *Predicate refinement*, the
per-shape table): `$x eq LIT` the exact value on the true edge and
`$x ne LIT` on the false one; `==` and `!=` the same where `LIT` is no
number under the target's numeral grammar, so the comparison is a string
one, and otherwise the type `Numeric` and, for an integer, the range
point — never the value, since `1.0 == 1` holds; a leading-zero literal
the target's release leaves open states nothing. `$x in LIST` states the
list's finite set, split under the target's word rules, and `ni` the same
on its false edge; `$x < N`, `<=`, `>` and `>=` against an integer numeral,
either side, the half-line an integer `x` lies on, on each edge, while a
boolean word, which a comparison reads as a string, a double and a bignum
state nothing; `[string is CLASS -strict $x]` the representation the
test leaves its value with (`tcl_registry::commands::tcl::string_is_member_type`:
`Int` for the integer classes and `Numeric` for `double` under `-strict`,
`Dict` for `dict` with or without it; `list` leaves `{}` a pure string and
the boolean classes leave `1` an integer, so neither proves a type);
`[info exists x]` and `[array exists x]` the existence
(§ *The existence rung*). `!` swaps the edges, `&&` keeps both true-edge
answers and `||` both false-edge ones, a left operand's only when the right
one changes no place. The variable is a plain local read whole — `$x`,
`${x}`, the flattened `switch` subject — and the other side a literal, so
`$x` alone, `$x eq $y`, an element and a qualified name state nothing. An
exact `switch` with no fall-through arm lowers to a chain of `eq` tests,
whose edges refine each arm; any other `switch` is one statement whose arms
are no edges.

The existence rung takes its own domain's facts. Every other domain's is
recorded for a variable no other actor may write — none externally
mutable, none an element of an escaping array, none linked to state
another invocation, the object or the host holds — and for none once a
computed name anywhere in the function may write or destroy a place. It
names the version the branch block leaves the variable at and holds at a
block where every executable edge into it carries it: a branch edge that
takes it, or an edge from a block where it holds. The solver computes that
as it sweeps (`RefinementFlow`): it enters each block with what the
executable edges into it carry and sweeps again while a block's set moves,
and since an edge, once executable, stays so, the sets only shrink, to the
greatest fixed point the edges into a cycle allow. An exception edge
carries none, and a block holding a barrier or an up-frame, which may
write any variable without a new version, holds none and passes none on.

A refinement never makes a version. For a block's statements and its
terminator the solver narrows each version an exact value or a finite set
in force there names (`narrow_values`, `refined_value`): a version the
lattice cannot pin takes the refinement's value or set, a set keeps the
members it allows, a constant stays itself, and the version's own value
comes back once the block is done. A φ reads its incoming versions' own
values, so a merge that two arms refining `x` to `a` and to `b` meet at
holds neither, and the post-passes — the constant branches, the template
plans and the selections — read each block as the sweep did. So `if {$x
eq "b"}` inside `if {$x eq "a"}` decides false: the solver opens only its
false edge, I230 reports it and the optimiser folds it. A definition in the
block reads the narrowed value, so `set y $x` in that arm is the constant
`a`, which O100 inlines where `y` is read; a read of `x` itself keeps the
version's own value.
`SccpResult::refinements_at` and `refinements_in(block)` name the
refinements in force at each block, and `value_at(block, key)` the value a
version holds there (`value_entries`); the Explorer's `sccp` view prints
each refinement on an executable edge (`refinement x = 'a'`, with its edge,
version and domain).

The type lattice reads the type refinements in force at a block
(`type_infer::type_refined_statements`, and the shimmer checks through
`type_infer::types_in_force`): in the arm of `[string is integer -strict
$x]` the version is an integer and in that of `$x == 1` a number, and its
own type again past the arm. The ranges read the range refinements in
place of a reading of the condition of their own
(`intervals::refine_interval`), and through them W230 to W233 narrow an
index or a divisor. A range from a comparison holds of a value that is an
integer — one that is not compares as a double or a string, so `end` passes
`$i > 5` and `7.0` fails `$i != 7` — so the ranges take a range refinement
only for a version proved an integer at the block (`proved_integer`): the
type lattice types it one (an integer literal, an `incr`, a route that
builds an integer), or a `string is integer -strict` refinement in force
there does. A numeric `==` proves a number, which `7.0` is, and no
integer.

### Preserve outcomes (`SccpResult::preserved`)

A definition whose statement left its place untouched — a
destructuring writer's declared `Preserve` outcome, such as `regexp` /
`scan` / `binary scan` on a no-match or an exhausted field, or a pack
command's declared `write_or_preserve` — is recorded in
`SccpResult::preserved`, keyed by the definition and mapped to the
*version before the statement*: the block's latest earlier definition,
else the block's entry version, else the root. A `<cond>` statement (the
condition immediately before a branch) gets no outcome of its own; the
definitions it reads are preserved when the shared engine decided that
branch and every nested command it ran answered without a store.

`analyser/diagnostics/dataflow.rs`'s undef trace (W210 on a read, a
`return`, or a condition's no-match arm; W213 on an `unset`) reads
through a preserved definition to the version `SccpResult::preserved`
names, replacing the private `regexp` / `scan` no-match prover: a no-match no longer needs its own read-before-set logic, only
the general trace over the fact every declared preserve outcome states.
A nested conditional writer (`regexp` in a word or a condition) records
its targets as read the same way the statement form does
(`ir_helpers::variable_write_effects_from_commands`), so the optimiser
never deletes the store a no-match preserves there either; and a write in
a `catch` or `try` body inside a substitution is read the same way, since
the body may stop before it. A command that raises preserves the places
its error did not reach, and `scan`, which goes on past a store it cannot
make, preserves the place of each store that failed.

### Completion paths

The solver publishes a statement's values and existence per completion
path. A block a handler's region holds is a point a throw may leave from,
and a statement in it that certainly raises — its route answers `Error {
written, .. }`, the prefix rule of [value-transfers.md](value-transfers.md)
§ *`catch`, `try`, and completion* — answers `DefValues::Raised`: its
definitions take what the stores that ran left, and the places the error
did not reach keep what they held. An error in a word is an error before the
command, and the pair of an embedded call and its host that raises gives the
writes it made. The block's normal way out is then closed (`BlockExit`), and
a later statement of the block, which never runs, leaves its places as they
were. Outside a handler's region nothing is claimed of a raise.

A handler is entered over the exception edges from its region's points and
over its region entry (`Function::region_entries`, `RegionEntry`), the edge
from the block before a `catch` or `try` body, which carries the state
before the body. The solver opens a region entry once the body's first
command is processed and does not raise only after a store
(`BlockExit::entry_throws`), so no handler sees the state before `lassign
{new second} a b` over an array `b`, which writes `a` first. A `try` body
that never rests has the entry where its first statement may fail before it
stores and is no literal assignment, which raises only where its own place
holds an array. The existence rung enters a handler with the join of its
region's points. A flattened `catch` is evaluated where its region ends
(`Function::catch_ends`, `LatticeDriver::evaluate_catch_end`) through the
`catch` route, over the versions the block before the body exits with, so
its result variable holds what the script returned; a `catch` that stays one
call is evaluated as any other call, and one inside a `[…]` substitution with
its statement's word effects. O109 and O126 read the region entries the
solver opens and the definitions a raise preserved
([optimisation-passes.md](optimisation-passes.md)).

### Unreachable blocks

Blocks that are never reached (due to constant branches, code after
`return`/`break`, etc.) are the complement of `SccpResult.executable_blocks`
— `FunctionUnit.sccp`, the return value of `sccp()`.  The optimiser derives
the set with `unreachable_blocks(&fu.cfg, &fu.sccp)`
(`rust/tcl-compiler/src/optimiser/elimination.rs`).  Taint analysis and
optimisation passes skip unreachable blocks.

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
(`rust/tcl-compiler/src/dead_stores.rs`) for dead stores.

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
straight off the `FunctionUnit`'s SSA and def-use facts → diagnostic **W210**.

Existence checks are excluded: `info exists X` / `array exists X` test a
variable rather than reading its value, so the check reference itself is never
a read-before-set.  `existence_query_vars`
(`rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`) recognises both the
bare-call form (`info exists X`) and the command-substitution form
(`set y [info exists X]`, `puts [array exists X]`).

A check also narrows the region it guards.  The condition's transfer states
the check's facts on its edges (§ *Edge refinements*): the true edge of
`[info exists X]` proves `X` bound, `![info exists X]`'s false edge the same.
The solver records each such place with the block the edge enters
(`SccpResult::existence_guards`), whether or not the existence rung refines
the place, and W210 takes the guard's word in every block that block
dominates (`SccpResult::guarded`): a read there, a φ incoming whose
predecessor lies there (`phi_can_undef`), a `return` there.  The rung
refines no place another actor may write — in a module whose callbacks the
analyser cannot read, none at all — so for such a place the guard is the
only evidence W210 has that the check ran.  The opposite branch is no part
of the guard's region, so a read there is still flagged.  Narrowing is a
runtime fact (the guard passed), so unlike the fold it needs no
foldability gate.

Only the exact three-word forms are recognised.  `existence_query::in_text`
requires exactly `info exists NAME` or `array exists NAME`, as the registry
resolves the invocation (a rooted `::info exists NAME` among them); the
queried word is taken verbatim, with no name-shape test of its own.  The
existence rung applies its own place gate — its refinements skip a place
another actor may write (`existence_refinements`) — and the guards W210
reads apply none (`existence_guards`, `SccpResult::guarded`).  Membership
idioms (`[info vars X]` / `[info locals X]` compared with `""`, `[llength
[info vars X]]`, `[lsearch [info vars] X] > -1`) and `catch {set _ $X}` are
**not** recognised as existence proofs.

### Unused variables

Variables that are defined but never read (across all versions) are reported
by `emit_unused_variable_diagnostics`
(`rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`), again from the
`FunctionUnit` → diagnostic **W211**.

### Worked example — `set x 5; if {$x < 0} {…} elseif {$x > 0} {…} else {…}`

SCCP determines `x₁ = Const(Int(5))`:
- `5 < 0` → `false` → the `entry_1 → if_then_3` edge is not executable
- `5 > 0` → `true` → `if_then_5` taken, `if_next_6` (the `else` body) not executable
- `sign` resolves to `Const(Int(1))` — only one reachable definition reaches its phi

### Worked example — `set i 0; while {$i < 5} { incr i }`

- `i₁ = Const(Int(0))` (before loop)
- `i₂ = phi(i₁, i₃)` at `while_header_2` → `Overdefined` (loop-carried)
- `evaluate_def` folds `Statement::Incr` through the registry's `incr`
  route only when the target's current value is exact; `i₂` is
  `Overdefined`, so the route declines, `i₃` is `Overdefined` too, and the
  phi cannot recover

## Decision rule

- If a value should be constant but is `Overdefined`, check whether a
  loop phi or barrier is widening it — or whether the defining statement is
  simply not a shape `evaluate_def` folds.
- `evaluate_def` folds `Statement::AssignConst`, `Statement::AssignExpr`
  through the expression evaluator, a `Statement::AssignValue` whose RHS is a
  literal / lattice-constant `$var` / foldable `[cmd …]`, and a
  single-variable single-list `foreach` (to the `ConstSet` of its elements).
  A command's value — the typed `Statement::Incr`, a call such as `append`,
  `lappend`, `set` or a `dict` keyed update, and a `[cmd …]` such as
  `[string range …]`, `[llength …]` or `[set x]` — comes through the
  registry's declared route for the resolved invocation
  (`value_transfer.rs`, [value-transfers.md](value-transfers.md)): the
  route reads its operands from the lattice and answers under the target's
  release, one evaluated write gives the call's single definition its
  value, and a decline — an inexact operand, a release-ambiguous axis
  under a profile naming no release, a head the module rebinds — leaves it
  `Overdefined`. Every statement's route and answer is in
  `SccpResult::explanations`, which the Explorer's `sccp` view prints,
  beside `SccpResult::route_tally`'s per-family entry counts (`routes
  entered: direct N · expression M · implementation K`), nested route
  entries included. The tally is zeroed at the top of each sweep, so only
  the settled sweep's entries survive, and the branch-fold pass that runs
  once after the fixed point adds its own, which is why one decided
  `if {1} {…}` counts as two expression entries. Every other statement kind — and every
  `Statement::Barrier` — widens its defs to `Overdefined`.
- `Statement::AssignExpr` and a value-position `[expr …]` run the shared
  expression engine (`tcl_expr_eval::evaluate_expression`) over the
  lattice inputs: `var` reads the proven value, `command` resolves a
  nested invocation through the registry under an effect-free policy, and
  `call` proves a `::tcl::mathfunc` binding; the answer is the engine's
  full value, so a string result folds too. `evaluate_branch` resolves a
  condition the same way and, when exactly one distinct SSA value among
  its reads is `Finite`, decides once every member agrees — two distinct
  finite reads decline `CorrelatedSets` instead of pairing arbitrarily.
- Liveness is computed backward from uses to definitions — if a new IR
  node reads variables, ensure they appear in `SsaStatement::uses`.
- SCCP runs once per function (no iterative refinement across functions —
  that is interprocedural analysis), though the optimiser's propagation pass
  re-runs it with `sccp_with_builtin_folds` / `sccp_with_extra_escaping` when
  it needs a projection the shared per-unit lattice cannot carry.

## Related docs

- [Examples 3–7 in walkthroughs](../../../docs/design/compiler/example-walkthroughs.md#example-3-expr-2--3)
- [GLOSSARY.md — SCCP, Lattice, Liveness](../../GLOSSARY.md#sccp)
- [cfg-ssa-fact-model.md](cfg-ssa-fact-model.md)
- [downstream-pass-contracts.md](downstream-pass-contracts.md)
- [value-transfers.md](value-transfers.md) — the outcome kinds (`Write` /
  `Preserve` / `Unbind` / `MayWrite` / `WriteElement`), `FoldedType` and
  the template-word plan every `SccpResult` field above answers from

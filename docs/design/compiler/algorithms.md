# Algorithms in the Tcl compiler/analyser — references & adaptations

The static-analysis pipeline (`compiler/`) is built on a small set of classic
compiler algorithms.  This document records, for each, the **original
reference** (every citation below was verified to exist with the stated
authors/venue/year) and **how it is adapted for Tcl** — a dynamically-typed,
dynamically-dispatched language with command substitution, `upvar`/`uplevel`
aliasing, and dialect/stub-driven command semantics.

The recurring Tcl adaptation theme: where a classic algorithm assumes a static
call/variable structure, we insert **barriers** (opaque points where an unknown
command may do anything) and treat **externally-mutable names**
(`upvar`/`global`/`namespace`/`::`-qualified/traced) conservatively, so every
result stays *sound* under Tcl's runtime dynamism.

---

## Static Single Assignment (SSA) & dominance frontiers

**Reference.** R. Cytron, J. Ferrante, B. K. Rosen, M. N. Wegman, F. K. Zadeck,
"Efficiently Computing Static Single Assignment Form and the Control Dependence
Graph," *ACM TOPLAS* 13(4):451–490, 1991 (doi:10.1145/115372.115320).

**Where.** `rust/tcl-compiler/src/ssa.rs` (`build_ssa`, `compute_phi_vars`,
the `RenameWalk` dominator-tree rename).

**Adaptation.** Phi placement uses the iterated dominance frontier exactly as in
Cytron et al.  Tcl-specific: variables that escape the frame
(`upvar`/`global`/`variable`/`namespace upvar`, traced vars) are detected via
the shared `rust/tcl-compiler/src/var_scoping.rs` grammar and handled conservatively by
downstream passes rather than being renamed away; command substitutions inside
words are descended for variable *reads* (`rust/tcl-compiler/src/var_refs.rs`) so SSA use
sets are not blind to `[expr {$x}]`-style hidden references.

## Dominators

**Reference.** K. D. Cooper, T. J. Harvey, K. Kennedy, "A Simple, Fast Dominance
Algorithm," Rice University TR, 2001 (the iterative O(N²)-in-practice
dominator/df formulation).

**Where.** `rust/tcl-compiler/src/ssa.rs` (`compute_idom_fast`,
`compute_dominance_frontier`, `build_dom_tree`),
`rust/tcl-compiler/src/loops.rs` (`dominates`).

**Adaptation.** Used as published over the per-function CFG.  The CFG itself is
Tcl-shaped: control structures (`if`/`while`/`for`/`foreach`/`switch`/`catch`/
`try`) are lowered to blocks, and any command we cannot reason about lowers to
a **`Statement::Barrier`** node that conservatively breaks dataflow.

`compute_idom_fast` is the production path: it works on reverse-postorder
indices and one `idom` pointer per block, so it is O(N·D) time and O(N)
memory.  The set-based `compute_dominators` + `compute_idom` pair is kept
only as the reference implementation the fast path is cross-validated
against (`#[cfg(test)]`), because materialising the dominator *sets* is
O(N²) memory on a multi-thousand-branch generated proc.

Dominance is queried two ways.  The default is a walk up the `idom` chain
(`loops::dominates`, `intervals::dominates`,
`diagnostics::helpers::block_dominated_by`), which is O(depth).  On a flat
N-branch dispatch chain the chain *is* the whole function, so a per-block-pair
loop over it is O(V²); `SsaFunction::dominator_intervals` answers the same
question in O(1) from a pre-order DFS numbering of the dominator tree
(half-open `[enter, exit)` intervals, nesting = dominance).  GVN's
loop-invariant scan is the one consumer that needs it.

## Natural-loop forest

**Reference.** Standard back-edge / natural-loop construction (a back edge
`tail → header` where *header* dominates *tail*; the loop body is the set of
nodes reaching *tail* without leaving through *header*) — as in the dragon book
and the dominance literature above.

**Where.** `rust/tcl-compiler/src/loops.rs` (`build_loop_forest`, `NaturalLoop`,
`LoopForest`).

**Adaptation.** Built from the SSA `idom` map over the SCCP-executable
blocks only, so an unreachable back edge never invents a loop.  A
`NaturalLoop` records block *names* (`header`, sorted `blocks`, sorted
`latches`) rather than `BlockId`s, and `LoopForest` orders loops by
header first-encounter, so the output is deterministic.  Nothing caches it
on the `FunctionUnit`; it is rebuilt per call.

`build_loop_forest` is **not** the only back-edge detector in the tree, and
the others are not thin wrappers over it:

| Detector | Where | Why it is separate |
|---|---|---|
| `build_loop_forest` | `loops.rs` | Full forest (header + body + latches); read by the read-before-set phi-undef helper in `analyser/diagnostics/helpers.rs` |
| `loop_headers` | `intervals.rs` | Only needs the *header set*, to pick widening points |
| `find_loop_invariants`' own back-edge scan + `natural_loop_blocks` | `gvn.rs` | Needs per-header block sets keyed by `BlockId` and uses `dominator_intervals` for O(1) dominance |
| `loop_body_blocks` | `shimmer/graph.rs` | An any-cycle detector: it must count a block reachable only through a `try`→handler exception edge as "in a loop", which the dominance-based natural-loop construction classifies differently |

Unifying them is not a free rename — each answers a different question over
a different block keying, and the shimmer one deliberately disagrees.

## Sparse Conditional Constant Propagation (SCCP)

**Reference.** M. N. Wegman, F. K. Zadeck, "Constant Propagation with Conditional
Branches," *ACM TOPLAS* 13(2):181–210, 1991 (doi:10.1145/103135.103136).

**Where.** `rust/tcl-compiler/src/sccp.rs` (`sccp`, `join`, `SccpResult`),
`rust/tcl-compiler/src/analyses.rs` (`LatticeValue`, `ConstValue`).

**Adaptation.** The optimistic constant lattice and executable-edge worklist are
as published, extended with a `ConstSet` (small finite value-set) kind for Tcl's
common multi-valued constants; `join` widens a set past its size cap to
`Overdefined`.  Two Tcl-critical soundness rules:
(1) a **`Statement::Barrier`** widens every tracked value to `Overdefined` (an
unknown command may mutate anything); (2) **externally-mutable names** (global /
namespace / `upvar`-aliased / traced — `sccp::is_externally_mutable` plus the
whole-module `TraceInputs`) are forced `Overdefined` and never folded,
because they are shared state writable from other scopes, traces, and source
files — folding through them is unsound across any opaque call (e.g.
`set ::g 5; mut; $::g` must not fold to 5).

`sccp` itself cannot decide an `[info exists X]` predicate — it is an opaque
`ExprNode::Command`, and SCCP holds neither parameter nor existence facts —
so `existence_constant_branches` runs as a post-pass with the frame's own
facts and contributes extra `ConstantBranch` entries.

## Semi-pruned SSA (φ-reduction)

**Reference.** P. Briggs, K. D. Cooper, T. J. Harvey, L. T. Simpson, "Practical
Improvements to the Construction and Destruction of Static Single Assignment
Form," *Software: Practice and Experience* 28(8):859–881, 1998.

**Where.** `rust/tcl-compiler/src/ssa.rs` (`nonlocal_names_and_defsites`,
`compute_phi_vars`).

**Adaptation.** The non-local (upward-exposed-use) set is computed over the Tcl
CFG including branch-condition and `return`-value reads, in the same pass that
collects def-sites.  `compute_phi_vars` then runs the iterated dominance
frontier over the def-sites of non-local names only, skipping any name no
block reads before redefining it — a phi for a purely-local name has no
reader, so dropping it removes only dead phis and leaves every
use/value/liveness/diagnostic result unchanged, with a smaller
SCCP/type/liveness/taint fixpoint.
Placing fewer phis is only safe once every read is tracked: a read the use
tracker misses makes an omitted phi look like a dead store.  The structural read
recovery in `rust/tcl-compiler/src/place_bridge.rs` closes that gap — reads
hidden inside command-substituted `expr` bodies and other nested forms are
recovered — so dropping the now-dead phis surfaces no latent false positive.

## Abstract interpretation — interval domain with widening/narrowing

**Reference.** P. Cousot, R. Cousot, "Abstract Interpretation: a Unified Lattice
Model for Static Analysis of Programs by Construction or Approximation of
Fixpoints," *POPL* 1977, pp. 238–252 (introduces the widening/narrowing
framework that guarantees termination over infinite-height domains).

**Where.** `rust/tcl-compiler/src/intervals.rs` (`Interval`, `widen`, `compute_intervals`,
`refine_interval`).

**Adaptation.** A small integer-interval domain runs **after** SCCP (it reads,
but does not perturb, the constant lattice — a strictly parallel analysis so no
existing diagnostic changes), seeded from SCCP constants, propagated forward in
reverse post-order with **widening at loop headers** so loop-induction values
terminate at `[0, +inf)` instead of an iteration cap.  The header set comes from
`intervals::loop_headers`, its own back-edge scan (`u → v` where `v` dominates
`u`) rather than the `LoopForest`, because only the header *set* is needed.
Constant-bound branch guards narrow via dominator-implied constraints
(`build_guard_index` + `refine_interval`: `if {$i < 10}` ⇒ `i ∈ [lo, 9]` in the
dominated region).  A *symbolic* bound (`$i < [llength $l]`) is left
unrefined — `guard_constraint` requires a literal integer on one side, and a
non-relational interval domain cannot relate the index to the list length: a
deliberately-documented precision limit.

The fixpoint is capped at 50 passes.  If it has not converged the result may
still be ascending — intervals *narrower* than reality — so every value is
degraded to `TOP` rather than letting a consumer derive a finding from an
under-approximation.

## Global Value Numbering (GVN)

**Reference.** B. Alpern, M. N. Wegman, F. K. Zadeck, "Detecting Equality of
Variables in Programs," *POPL* 1988 (the value-partitioning basis of value
numbering).

**Where.** `rust/tcl-compiler/src/gvn.rs`.

**Adaptation.** Value numbering plus loop-invariant code-motion hints.
`find_loop_invariants` derives its own header → block-set map from a back-edge
scan over the executable blocks, resolving dominance through
`SsaFunction::dominator_intervals` (O(1) per query), because it needs
`BlockId`-keyed sets rather than the name-keyed `LoopForest`.  "Executable"
there is `SccpResult::executable_blocks` — the same set the deletion passes
read (`optimiser/elimination.rs`'s `unreachable_blocks`, the O107 source) —
passed in by the caller, never re-derived as raw CFG reachability: a hoist
offered into a block behind a constant-false branch would target code the
optimiser is simultaneously offering to delete (issue #1385).
`find_partial_redundancies` takes the same set, and both `*_for_function`
entries seed it from `FunctionUnit::sccp`.  Only
side-effect-free expressions are numbered/hoisted — purity is decided by
`side_effects::classify_side_effects` through `gvn::is_pure_command` /
`is_pure_with_procs`, with `find_pure_procs` extending it to user procedures —
and barriers and impure calls reset value equivalences, so dynamic dispatch
never produces an unsound equivalence.

## Linear-scan / graph-colouring slot allocation

**References.** G. J. Chaitin, "Register Allocation & Spilling via Graph
Colouring," *SIGPLAN Symp. on Compiler Construction*, 1982 (interference-graph
colouring); M. Poletto, V. Sarkar, "Linear Scan Register Allocation," *ACM
TOPLAS* 21(5):895–913, 1999 (single-pass live-range allocation).

**Where.** `rust/tcl-compiler/src/slot_allocation.rs` (`build_interference`,
`coalesce_slots`).

**Adaptation.** Coalesces *variable-name* slots (not machine registers): two
names whose SSA live ranges do not overlap share a slot.  Interference is
computed **instruction-granular** (a backward walk per block seeded from
`live_out` plus the terminator's reads), which is what lets straight-line Tcl
locals with disjoint ranges share a slot.  Parameters are pinned to stable low
slots.  Applies only to the WASM emitter and an opt-in bytecode mode; the
default bytecode path stays one-slot-per-name for tclsh byte-parity.

## Worklist dataflow (liveness, type/taint propagation)

**Reference.** G. A. Kildall, "A Unified Approach to Global Program
Optimization," *POPL* 1973 (the monotone-framework fixpoint that underlies the
worklist solvers).

**Where.** `rust/tcl-compiler/src/slot_allocation.rs` (`live_out_by_name`) and
`rust/tcl-compiler/src/dead_stores.rs` (liveness),
`rust/tcl-compiler/src/type_infer.rs` (`propagate_types`),
`rust/tcl-compiler/src/rendered_properties.rs` (`propagate_rendered_props`),
`rust/tcl-compiler/src/taint.rs`.  `analyses.rs` holds only the shared lattice
types.

**Adaptation.** Standard monotone worklists over the SSA value graph, sharing one
reverse-postorder (`cfg::Function::reverse_postorder`, iterative so a
multi-thousand-block CFG cannot overflow the worker stack) and the SCCP
executable-block/edge sets so unreachable code is not analysed.  Liveness runs
backward over the same order reversed, keyed by variable **name** rather than
`(name, version)`: slots are per-name, so dropping versions makes phi renaming
across an edge a no-op.  Tcl barriers act as ⊤-introducing transfer functions.

## Per-proc incremental memoization

**Basis.** Not a single classic paper — the standard incremental/demand-driven
recomputation idea: recompute a unit only when its inputs change.

**Where.** `rust/tcl-compiler/src/compilation_unit.rs`; the salsa-backed
query layer in `rust/tcl-lsp-db/src/lib.rs`.

**Adaptation.** The unit of reuse is the per-proc `FunctionUnit`, memoised by
`tcl-lsp-db`'s `function_lattice` query on an interned `FnLatticeKey`: the
procedure's **post-inline IR body normalised to offset 0**, its qualified name
and parameter list, the interned module-wide `CfgContext` (the `upvar_procs` /
`proc_params` / `global_write_procs` maps from `prepare_cfg_context`), the
dialect, any interprocedural SCCP parameter seeds, the sorted known-class list,
and the module's variable-trace facts.  The body's **position is deliberately
not in the key** — a shifted-but-unchanged body interns to the same key and the
builder rebases the cached result to the body's real span through
`FunctionUnit::base_offset`.  Cross-proc invalidation rides the `CfgContext`
and the whole-module fact fields.  Every key component is an
over-approximation, which is the safe
bias: a key that changes more often than strictly necessary forces extra
recomputation, but can never serve a stale result.  The hard contract on any
change here is that incremental analysis must equal a full rebuild
byte-for-byte.

---

*Citations were verified against the ACM Digital Library / publisher records and
authors' copies at the time of writing.*

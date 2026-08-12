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

**Where.** `rust/tcl-compiler/src/ssa.rs` (`build_ssa`, `_phi_vars`, renaming).

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

**Where.** `rust/tcl-compiler/src/ssa.rs` (`_compute_idom_fast`, `_dominance_frontier`),
`rust/tcl-compiler/src/loops.rs` (`dominates`).

**Adaptation.** Used as published over the per-function CFG.  The CFG itself is
Tcl-shaped: control structures (`if`/`while`/`for`/`foreach`/`switch`/`catch`/
`try`) are lowered to blocks, and any command we cannot reason about lowers to
an **`IRBarrier`** node that conservatively breaks dataflow.

## Natural-loop forest

**Reference.** Standard back-edge / natural-loop construction (a back edge
`tail → header` where *header* dominates *tail*; the loop body is the set of
nodes reaching *tail* without leaving through *header*) — as in the dragon book
and the dominance literature above.

**Where.** `rust/tcl-compiler/src/loops.rs` (`build_loop_forest`, `NaturalLoop`,
`LoopForest`).

**Adaptation.** The single natural-loop source for GVN/LICM and the interval
domain's widening points, built from the SSA dominator info.  Because dominance
queries are O(1) (Euler-tour interval labels on the dominator tree, see above),
re-deriving the forest per consumer is cheap, so it is recomputed rather than
cached on the `FunctionUnit`.  Shimmer keeps a *separate* any-cycle detector
(`rust/tcl-compiler/src/shimmer/:_loop_body_blocks`): it must count a block reachable only
through a `try`→handler exception edge as "in a loop", which the dominance-based
natural-loop construction classifies differently (the two sets diverge on ~0.3%
of corpus functions — all `try`/coroutine bodies), so the two detectors are
intentionally not unified.

## Sparse Conditional Constant Propagation (SCCP)

**Reference.** M. N. Wegman, F. K. Zadeck, "Constant Propagation with Conditional
Branches," *ACM TOPLAS* 13(2):181–210, 1991 (doi:10.1145/103135.103136).

**Where.** `rust/tcl-compiler/src/analyses.rs` / `rust/tcl-compiler/src/sccp.rs` (`_sccp`, `LatticeValue`, `_join`).

**Adaptation.** The optimistic constant lattice and executable-edge worklist are
as published, extended with a `CONSTSET` (small finite value-set) kind for Tcl's
common multi-valued constants.  Two Tcl-critical soundness rules:
(1) an **`IRBarrier`** widens every tracked value to `OVERDEFINED` (an unknown
command may mutate anything); (2) **externally-mutable names** (global /
namespace / `upvar`-aliased / traced) are forced `OVERDEFINED` and never folded,
because they are shared state writable from other scopes, traces, and source
files — folding through them is unsound across any opaque call (e.g.
`set ::g 5; mut; $::g` must not fold to 5).

## Semi-pruned SSA (φ-reduction)

**Reference.** P. Briggs, K. D. Cooper, T. J. Harvey, L. T. Simpson, "Practical
Improvements to the Construction and Destruction of Static Single Assignment
Form," *Software: Practice and Experience* 28(8):859–881, 1998.

**Where.** `rust/tcl-compiler/src/ssa.rs` (`_nonlocal_names_and_defsites`, `_phi_vars`).

**Adaptation.** The non-local (upward-exposed-use) set is computed over the Tcl
CFG including branch-condition and `return`-value reads, in the same pass that
collects def-sites.  **Shipped** — phis are placed only for non-local names
(≈20% fewer phis on the corpus, so smaller SCCP/type/liveness/taint fixpoints).
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
existing diagnostic changes), seeded from SCCP constants, propagated forward
with **widening at `LoopForest` loop headers** so loop-induction values
terminate at `[0, +inf)` instead of an iteration cap.  Constant-bound branch
guards narrow via dominator-implied constraints (`if {$i < 10}` ⇒ `i ∈ [lo, 9]`
in the dominated region).  A *symbolic* bound (`$i < [llength $l]`) is left
unrefined — a non-relational interval domain cannot relate the index to the list
length, a deliberately-documented precision limit.

## Global Value Numbering (GVN)

**Reference.** B. Alpern, M. N. Wegman, F. K. Zadeck, "Detecting Equality of
Variables in Programs," *POPL* 1988 (the value-partitioning basis of value
numbering).

**Where.** `rust/tcl-compiler/src/gvn.rs`.

**Adaptation.** Value numbering plus loop-invariant code-motion hints, consuming
the shared `LoopForest`.  Only side-effect-free (`rust/tcl-compiler/src/side_effects.rs`
pure) expressions are numbered/hoisted; barriers and impure calls reset value
equivalences, so dynamic dispatch never produces an unsound equivalence.

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

**Where.** `rust/tcl-compiler/src/analyses.rs` / `rust/tcl-compiler/src/sccp.rs` (liveness, type lattice, rendered-property
propagation), `rust/tcl-compiler/src/taint.rs`.

**Adaptation.** Standard monotone worklists over the SSA value graph, sharing one
reverse-postorder and the SCCP executable-block/edge sets so unreachable code is
not analysed.  Tcl barriers act as ⊤-introducing transfer functions.

## Per-proc dependency fingerprint (incremental memoization)

**Basis.** Not a single classic paper — the standard incremental/demand-driven
recomputation idea (recompute a unit only when its inputs change), with the
*dependency fingerprint* playing the role of the unit's input summary.

**Where.** `rust/tcl-compiler/src/compilation_unit.rs` (`dependency_fingerprint` — an
experimental foundation, **not wired into any cache**; the per-proc
`FunctionUnit` reuse that shipped keys on body + stub + CFG-context + position +
known-classes fingerprints in `document_state._build_proc_cache`, not on this
dependency fingerprint. Wiring it in is risk-gated by the incremental≡full
golden test).

**Adaptation.** A proc's fingerprint is the over-approximate set of external
symbols it references — invoked command names (including inside `[...]`
substitutions in nested control-flow bodies) and `::`-qualified globals — so a
proc's *local* diagnostics need recomputing only when its body hash or that set
changes.  Over-approximation is the safe bias: a missing dependency can force
extra recomputation but never serve a stale result.

---

*Citations were verified against the ACM Digital Library / publisher records and
authors' copies at the time of writing.*

# CFG/SSA fact model and consumers

The flow facts core analysis already computes, and the rule that a new pass
consumes them rather than re-deriving its own. Read this when designing a pass
that needs reachability, def-use, or type information.

CFG/SSA/core analyses already compute high-value facts (reachability, definitions/uses, type lattice states, dead stores, etc.). Duplicating this reasoning in each pass creates inconsistency risk.

## Preferred model

- Build facts once from `cfg::Function` + `ssa::SsaFunction`. The carrier is
  `FunctionUnit` (`rust/tcl-compiler/src/compilation_unit.rs`), built by
  `CompilationUnit::build_for()` for the top-level script and every procedure.
  `FunctionUnit::build` runs SSA → def-use → SCCP → type-propagation →
  rendered-properties → taint-propagation, in that order.
- Treat specialised passes as consumers of those facts plus pass-specific heuristics.
- Preserve stable block/value naming in pass outputs so diagnostics can reference related sites.

## Typical fact categories

- control-flow: unreachable blocks (the complement of `SccpResult::executable_blocks`) and constant branch outcomes (`SccpResult::constant_branches`), both on `FunctionUnit::sccp`,
- data-flow: per-statement `defs` / `uses` on each `SsaStatement`, and read-before-set signals (a use at SSA version 0),
- def-use chains: precise per-SSA-value definition-to-use mapping (`FunctionUnit::def_use`, an `Arc<DefUseResult>`),
- memory-SSA: versioned memory operations and alias sets for upvar/global/variable/namespace-upvar (`FunctionUnit::memory_ssa`) — `None` until a caller asks for it with `with_memory_ssa`,
- type-flow: `FunctionUnit::types` (unknown / known / shimmered / overdefined, with concrete Tcl type hints) and the joined `FunctionUnit::return_type`,
- rendered-value properties: `FunctionUnit::rendered_props`,
- taint: `FunctionUnit::taints`, replaced by the interprocedural re-run,
- name-space blindness: `FunctionUnit::dynamic_names`, the three-bit `DynamicNameBarrier` every abstaining consumer reads in `O(1)`.

Liveness has **no** stored per-function map: each consumer derives what it
needs — `slot_allocation::live_out_by_name` for slot interference,
`dead_stores::liveness_dead_stores` for dead stores.  So is the natural-loop
forest (`loops::build_loop_forest`), which is rebuilt per call.

## Practical checklist

1. Can this new pass consume the `FunctionUnit` instead of recomputing flow?
   `analyses.rs` is where the shared lattice types — `LatticeValue`,
   `ConstValue`, and `LatticeKind` — live.
2. Does it honour `FunctionUnit::complexity_guarded`?  A guarded unit carries a
   trivial SSA shell, and every per-proc diagnostic and optimiser pass must skip
   it rather than read empty lattices as fact.
3. Does it recover absolute positions through `FunctionUnit::abs_span` /
   `abs_pos`?  A memoised procedure's unit is built at offset 0 and carries its
   real body offset in `base_offset`.
4. Does output carry source ranges and related ranges?
5. Are warnings stable enough for deterministic tests?

## Related files

- `rust/tcl-compiler/src/analyses.rs` / `rust/tcl-compiler/src/sccp.rs`
- `rust/tcl-compiler/src/cfg.rs` / `rust/tcl-compiler/src/cfg_builder/`
- `rust/tcl-compiler/src/ssa.rs`
- `rust/tcl-compiler/src/def_use.rs`
- `rust/tcl-compiler/src/memory_ssa.rs`
- `rust/tcl-compiler/src/dataflow_graph.rs`
- `rust/tcl-compiler/src/compilation_unit.rs`
- `docs/design/compiler/def-use-chains.md`
- `docs/design/compiler/memory-ssa.md`
- `rust/tcl-compiler/src/shimmer/`
- `rust/tcl-compiler/src/optimiser/`

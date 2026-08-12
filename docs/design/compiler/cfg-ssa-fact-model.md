# KCS: CFG/SSA fact model and consumers

## Symptom

A pass re-implements flow reasoning that already exists in core analyses.

## Context

CFG/SSA/core analyses already compute high-value facts (reachability, definitions/uses, type lattice states, dead stores, etc.). Duplicating this reasoning in each pass creates inconsistency risk.

## Preferred model

- Build facts once from `CFGFunction` + `SSAFunction` via `analyse_function()`.
- Treat specialised passes as consumers of those facts plus pass-specific heuristics.
- Preserve stable block/value naming in pass outputs so diagnostics can reference related sites.

## Typical fact categories

- control-flow: unreachable blocks, constant branch outcomes,
- data-flow: defs/uses, read-before-set signals,
- def-use chains: precise per-SSA-value definition-to-use mapping (`FunctionAnalysis.def_use_chains`),
- memory-SSA: versioned memory operations and alias sets for upvar/global/variable (`FunctionAnalysis.memory_ssa`),
- type-flow: known/unknown/overdefined and concrete Tcl type hints,
- execution-intent: command-substitution shape, side-effect/escape classes, shimmer pressure.

## Practical checklist

1. Can this new pass consume `FunctionAnalysis` instead of recomputing flow?
2. Does output carry source ranges and related ranges?
3. Are warnings stable enough for deterministic tests?

## Related files

- `rust/tcl-compiler/src/analyses.rs` / `rust/tcl-compiler/src/sccp.rs`
- `rust/tcl-compiler/src/ssa.rs`
- `rust/tcl-compiler/src/def_use.rs`
- `rust/tcl-compiler/src/memory_ssa.rs`
- `rust/tcl-compiler/src/dataflow_graph.rs`
- `rust/tcl-compiler/src/compilation_unit.rs`
- `docs/design/compiler/execution-intent-model.md`
- `docs/design/compiler/def-use-chains.md`
- `docs/design/compiler/memory-ssa.md`
- `rust/tcl-compiler/src/shimmer/`
- `rust/tcl-compiler/src/optimiser/`

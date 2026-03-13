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
- type-flow: known/unknown/overdefined and concrete Tcl type hints,
- execution-intent: command-substitution shape, side-effect/escape classes, shimmer pressure.

## Practical checklist

1. Can this new pass consume `FunctionAnalysis` instead of recomputing flow?
2. Does output carry source ranges and related ranges?
3. Are warnings stable enough for deterministic tests?

## Related files

- `core/compiler/core_analyses.py`
- `core/compiler/ssa.py`
- `core/compiler/compilation_unit.py`
- `docs/kcs/compiler/kcs-execution-intent-model.md`
- `core/compiler/shimmer.py`
- `core/compiler/optimiser/`

# KCS: Pass/fact ownership matrix

## Symptom

Contributors are unsure which compiler pass owns a fact, where it is produced, and which diagnostics or optimisations depend on it.

## Operational context

Multiple passes consume overlapping `CompilationUnit` and `FunctionUnit` facts. Without an explicit ownership map, changes can accidentally duplicate diagnostics or break downstream assumptions.

## Decision rules / contracts

1. **Single primary owner per fact family**
   - Each fact family has one primary producer pass/module.
2. **Consumers do not silently redefine producer semantics**
   - Consumer passes may derive helper facts, but must not redefine producer contract shape without updating KCS + tests.
3. **Ownership changes require cross-pass validation**
   - If a fact producer changes, revalidate all listed consumers and diagnostics integration tests.

## Pass -> fact -> consumer matrix

| Producer pass/module | Primary facts produced | Typical consumers | Anchors |
|---|---|---|---|
| `core/compiler/lowering.py` | `IRModule`, structured IR statements, `Range` mappings | CFG builder, interprocedural analysis, diagnostics range mapping | `lower_to_ir()`, `IR*` nodes |
| `core/compiler/cfg.py` | `CFGModule` / `CFGFunction` blocks, terminators, loop structure | SSA builder, codegen, flow-sensitive diagnostics | `build_cfg_function()` |
| `core/compiler/ssa.py` | SSA versions, phi nodes, dominance metadata | Core analyses (SCCP/liveness/types), taint, optimiser/GVN | `build_ssa()` |
| `core/compiler/core_analyses.py` | constant lattice, unreachable blocks, dead stores, type lattice | optimiser, diagnostics-layer enrichment, shimmer/taint heuristics | `analyse_function()` |
| `core/compiler/interprocedural.py` | proc summaries (purity, call graph, constant return, parameter sensitivity) | optimiser (O103), interproc taint propagation | `analyse_interprocedural_ir()` |
| `core/compiler/optimiser/` | optimisation findings (`O100`–`O125`) | diagnostics aggregation, code-action surfaces | `find_optimisations()` |
| `core/compiler/gvn.py` | redundancy findings (`O105`, `O106`) | diagnostics aggregation, optimisation hint ranking | `find_redundant_computations()` |
| `core/compiler/taint/` | taint findings (`T100`–`T201`, `IRULE3xxx`) | diagnostics aggregation, security workflows | `find_taint_warnings()` |
| `core/compiler/shimmer.py` | shimmer findings (`S100`–`S102`) | diagnostics aggregation, performance guidance | `find_shimmer_warnings()` |
| `core/compiler/irules_flow.py` | iRules flow findings (`IRULE1xxx`–`IRULE5xxx`) | diagnostics aggregation for iRules dialect | `find_irules_flow_warnings()` |
| `lsp/features/diagnostics.py` | final LSP diagnostic projection, suppression policy application | LSP publish pipeline, async tiering scheduler | `get_diagnostics()` |

## File-path anchors

- `core/compiler/lowering.py`
- `core/compiler/cfg.py`
- `core/compiler/ssa.py`
- `core/compiler/core_analyses.py`
- `core/compiler/interprocedural.py`
- `core/compiler/optimiser/`
- `core/compiler/gvn.py`
- `core/compiler/taint/`
- `core/compiler/shimmer.py`
- `core/compiler/irules_flow.py`
- `lsp/features/diagnostics.py`

## Failure modes

- Two passes emit overlapping findings for the same semantic issue with different code families.
- Consumer pass assumes a producer fact invariant that no longer holds after refactor.
- Diagnostics aggregation treats derived facts as canonical and bypasses producer ownership.

## Test anchors

- `tests/test_compilation_cache.py`
- `tests/test_optimiser.py`
- `tests/test_gvn.py`
- `tests/test_taint.py`
- `tests/test_shimmer.py`
- `tests/test_irules_checks.py`
- `tests/test_diagnostics.py`
- `tests/test_diagnostic_phases.py`

## Discoverability

- [compiler KCS index](README.md)
- [downstream pass contracts](kcs-downstream-pass-contracts.md)
- [diagnostics integration](kcs-diagnostics-integration.md)

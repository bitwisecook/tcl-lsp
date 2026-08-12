# Pass and fact ownership matrix

Which compiler pass owns a fact, where it is produced, and which diagnostics or
optimisations depend on it.

Multiple passes consume overlapping `CompilationUnit` and `FunctionUnit` facts.
Without an explicit ownership map, a change can accidentally duplicate
diagnostics or break a downstream assumption.

## Contracts

1. **One primary owner per fact family.** Each fact family has exactly one
   producing module.
2. **Consumers do not silently redefine producer semantics.** A consumer may
   derive helper facts, but it must not change the shape of a producer's
   contract without updating this doc and the tests.
3. **Ownership changes require cross-pass validation.** When a producer changes,
   revalidate every listed consumer and the diagnostics integration tests.

## Producer → fact → consumer

All paths are relative to `rust/tcl-compiler/src/` unless stated otherwise.

| Producer | Primary facts produced | Typical consumers | Entry points |
|---|---|---|---|
| `lowering/` | `Module`, structured IR statements, `Range` mappings, TclOO method bodies | CFG builder, interprocedural analysis, diagnostic range mapping, method-purity summaries | `lower_to_ir`, `lower_to_ir_with_body_cache`, the `Ir*` nodes |
| `cfg_builder/` (types in `cfg.rs`) | `CfgModule` / `Function` blocks, terminators, loop structure | SSA builder, codegen, flow-sensitive diagnostics | `build_cfg`, `build_cfg_function` |
| `ssa.rs` | SSA versions, phi nodes, dominance metadata | SCCP, liveness, type inference, taint, optimiser, GVN | `build_ssa` |
| `sccp.rs`, `type_infer.rs`, `dead_stores.rs` (result types in `analyses.rs`) | constant lattice, unreachable blocks, dead stores, type lattice | optimiser, diagnostic enrichment, shimmer and taint heuristics, dataflow graph | `analyses::LatticeValue`, `TypeLattice` |
| `def_use.rs` | def-use chains (per-SSA-value definition → use mapping) | dead-store detection, unused-variable precision, copy propagation, dataflow graph | `build_def_use_chains` |
| `memory_ssa.rs` (storage places in `place.rs`, `place_bridge.rs`) | memory versions, alias sets (`upvar` / `global` / `variable`) | alias-aware DSE, GVN across aliases, taint through aliases | `compute_aliases`, `is_clobber` |
| `dataflow_graph.rs` | data-flow graph (nodes, edges, aliases per function) | compiler explorer, MCP tools, AI skills | `extract_dataflow_graph`, `extract_function_dataflow` |
| `interprocedural.rs`, `taint_interproc.rs` | proc summaries (purity, call graph, constant return, parameter sensitivity); TclOO method summaries | optimiser (O103; the O126 `my <method>` purity gate), interprocedural taint propagation | `build_interprocedural_analysis` |
| `optimiser/` | optimisation findings (`O100`–`O130`) | diagnostics aggregation, code-action surfaces | `optimise_unit` (`optimiser/manager.rs`) |
| `gvn.rs` | redundancy findings (`O105`, `O106`) | diagnostics aggregation, optimisation-hint ranking | `find_pure_procs`, the redundancy message builders |
| `taint.rs` | taint findings (`T100`–`T106`, `IRULE3xxx`) | diagnostics aggregation, security workflows | `find_taint_warnings`, `find_taint_warnings_for_cu` |
| `shimmer/` | shimmer findings (`S100`–`S102`, `S110`) | diagnostics aggregation, performance guidance | `find_shimmer_warnings_for_cu` |
| `irules_checks.rs` | iRules flow findings (`IRULE1xxx`–`IRULE5xxx`) | diagnostics aggregation for the iRules dialect | the `find_*_warnings` entry points |
| `rust/tcl-lsp-db/src/lib.rs` | final LSP diagnostic projection, suppression policy | LSP publish pipeline, async tiering scheduler | `project_diagnostics`, `compiler_check_diagnostics` |

## Failure modes

- Two passes emit overlapping findings for the same semantic issue under
  different code families.
- A consumer assumes a producer invariant that no longer holds after a refactor.
- Diagnostics aggregation treats a derived fact as canonical and bypasses
  producer ownership.

## Related docs

- [downstream-pass-contracts.md](downstream-pass-contracts.md)
- [diagnostics-integration.md](diagnostics-integration.md)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)

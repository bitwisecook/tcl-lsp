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
| `value_transfer.rs` (the registry's `value_transfer` module owns the declarations and the direct routes: the cell updates behind `incr`, `append` and `lappend`, the cell write behind `set`, the keyed updates of `dict` and their `::tcl::dict::` spellings, `string range`, `list`, `llength`, `string length` and, since slice 3, `format`; the expression route assembles `expr`'s argument words — `ExpressionRoute::assemble` — and runs them through the shared engine, `tcl_expr_eval::ExprServices`, over the same lattice inputs; since slice 4, a pack's own declared implementation runs through the same driver — `DeclaredSemantics::evaluate` in `tcl-registry`'s `pack_hooks`, over the bounded host behind `tcl_engine_api::Engine`) | one invocation's value transfer: the registry's structural plan, per-domain transfer, and exact evaluation over lattice-proved operands; one evaluated write applied to the call's definition; each statement's route and answer (`SccpResult::explanations`) and the run's per-family entry counts (`SccpResult::route_tally`); the per-module `AnalysisContextKey` every answer is keyed under, whose command trust is the lattice's `ObservedBindings` stance and a rewrite's `WholeModule` one, and whose `evaluator_revision` (slice 4, D94) names the declared-implementation evaluators the run served with, so a healthy and a host-absent worker never share a declared implementation's answer. Unavailable is `Overdefined`, with the `DeclineReason` recorded beside it | `sccp.rs`, which feeds the lattice O100, O102 and O109 / O126 read, and whose `evaluate_branch` decides a condition once every member of its one finite read agrees; `optimiser/chain_fold.rs` (O104, O130), the tail fold, `static_loops.rs`, `intervals.rs` (the descriptor's `IntegerAdd`), `optimiser/helpers/expr_simplify.rs` (O110's regrouping, type-guarded since slice 3) and W231 (`analyser/bounds_checks.rs`), each reading the resolved cell update or expression answer rather than a command name; the Explorer's `sccp` view; `tcl-lsp-db`'s memoised `function_lattice`, under the same context and stance, re-keyed by `tcl_lsp_db::EvaluatorEpoch` on a plan publish or a quarantine (slice 4, D104); the existence fold's unbind set | `LatticeDriver` (crate-private), `AnalysisContextKey`, `tcl_registry::value_transfer::CommandSemantics`, `ExpressionSource`, `RouteTally` |
| `def_use.rs` | def-use chains (per-SSA-value definition → use mapping) | dead-store detection, unused-variable precision, copy propagation, dataflow graph | `build_def_use_chains` |
| `memory_ssa.rs` (storage places in `place.rs`, `place_bridge.rs`) | memory versions, alias sets (`upvar` / `global` / `variable`) | alias-aware DSE, GVN across aliases, taint through aliases | `compute_aliases`, `is_clobber` |
| `dataflow_graph.rs` | data-flow graph (nodes, edges, aliases per function) | compiler explorer, MCP tools, AI skills | `extract_dataflow_graph`, `extract_function_dataflow` |
| `interprocedural.rs`, `taint_interproc.rs` | proc summaries (purity, call graph, constant return, parameter sensitivity); TclOO method summaries | optimiser (O103; the O126 `my <method>` purity gate), interprocedural taint propagation | `build_interprocedural_analysis` |
| `optimiser/` | optimisation findings (`O100`–`O130`) | diagnostics aggregation, code-action surfaces | `optimise_unit` (`optimiser/manager.rs`) |
| `gvn.rs` | redundancy findings (`O105`, `O106`) | diagnostics aggregation, optimisation-hint ranking | `find_pure_procs`, the redundancy message builders |
| `taint.rs` | taint findings (`T100`–`T106`, `IRULE3xxx`) | diagnostics aggregation, security workflows | `find_taint_warnings`, `find_taint_warnings_for_cu` |
| `shimmer/` | shimmer findings (`S100`–`S103`, `S110`) | diagnostics aggregation, performance guidance | `find_shimmer_warnings_for_cu` |
| `irules_checks.rs` | iRules flow findings (`IRULE1xxx`–`IRULE5xxx`) | diagnostics aggregation for the iRules dialect | the `find_*_warnings` entry points |
| `rust/tcl-lsp-db/src/lib.rs` | final LSP diagnostic projection | LSP publish pipeline, async tiering scheduler | `project_diagnostics`, `compiler_check_diagnostics` |
| `rust/tcl-lsp-core/src/diagnostic_policy.rs`, `diagnostic_report.rs` | diagnostic policy: directives applied, the five scopes, the seed, severity, optimiser and shimmer gates, overlaps, abstention; every finding kept with its reason | the LSP adapter, `tcl diag` / `lint` / `validate` / `opt`, the MCP tools, the code actions | `apply`, `PolicyBuilder`, `document_report` |

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

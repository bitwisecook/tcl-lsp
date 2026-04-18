# Compiler design docs

Focused, high-churn compiler design docs live in this folder. Each file
describes one piece of the pipeline — how it works, what it consumes and
produces, and the contracts that downstream consumers depend on.
Technical jargon is allowed here.

User-facing compiler troubleshooting and how-tos live in
[`docs/kcs/`](../../kcs/README.md).

## Start here

- [compiler-pipeline-overview.md](compiler-pipeline-overview.md) — stage
  map and fact hand-off boundaries.
- [compiler-systems-overview.md](compiler-systems-overview.md) —
  subsystem contract map for quick ownership triage.

## Pipeline stages

- [lexing-segmentation.md](lexing-segmentation.md) — token and command
  segmentation.
- [expression-parsing.md](expression-parsing.md) — Pratt parser, braced
  and unbraced expressions.
- [cfg-construction.md](cfg-construction.md) — basic block
  decomposition patterns.
- [ssa-construction.md](ssa-construction.md) — version numbering and
  phi placement.
- [ir-types-lowering.md](ir-types-lowering.md) — IR node selection
  rules.
- [lowering-dispatch.md](lowering-dispatch.md) — argument roles and
  command classification.
- [full-pipeline-walkthrough.md](full-pipeline-walkthrough.md) —
  end-to-end source to bytecode walkthrough.
- [control-flow-patterns.md](control-flow-patterns.md) — if, while,
  for, foreach, and proc compilation.
- [error-recovery.md](error-recovery.md) — virtual token injection for
  malformed input.

## Analysis

- [sccp-core-analyses.md](sccp-core-analyses.md) — constant propagation
  and liveness.
- [constant-folding-type-inference.md](constant-folding-type-inference.md)
  — SCCP and type lattice.
- [def-use-chains.md](def-use-chains.md) — def-use chain construction
  and consumer contracts.
- [memory-ssa.md](memory-ssa.md) — memory-SSA, alias detection, and
  versioned memory operations.
- [dataflow-graph.md](dataflow-graph.md) — data-flow graph extraction,
  serialisation, and consumer contracts.
- [rendered-value-properties.md](rendered-value-properties.md) — string
  content analysis over SSA.
- [taint-analysis.md](taint-analysis.md) — sources, sinks, colours, and
  propagation.
- [var-escape-analysis.md](var-escape-analysis.md) — which Tcl vars stay
  on WASM locals vs spill to the runtime frame.
- [interprocedural-analysis.md](interprocedural-analysis.md) —
  ProcSummary construction.
- [optimisation-passes.md](optimisation-passes.md) — pass table and
  priorities.

## Infrastructure

- [command-registry.md](command-registry.md) — command metadata, specs,
  arity, and taint hints.
- [data-structure-reference.md](data-structure-reference.md) — pipeline
  types at each stage.
- [connection-scope.md](connection-scope.md) — cross-event variable
  flow in iRules.
- [dialects-events.md](dialects-events.md) — dialect filtering and
  event requirements.
- [event-priority-model.md](event-priority-model.md) — base priority
  and offset model for event handlers.
- [namespace-resolution.md](namespace-resolution.md) — qualified name
  handling.
- [diagnostics-calculation.md](diagnostics-calculation.md) — two-phase
  diagnostic architecture.
- [codegen-internals.md](codegen-internals.md) — LVT, linearisation,
  labels, and peephole optimisation.

## Side-effects and effect classification

- [side-effects-system.md](side-effects-system.md) — structured
  side-effect hints, classification flow, and how to add hints to
  commands.

## Pipeline contracts

- [lowering-contracts.md](lowering-contracts.md) — lowering guarantees
  consumed by CFG, SSA, and codegen.
- [cfg-ssa-fact-model.md](cfg-ssa-fact-model.md) — core fact model and
  consumption rules.
- [execution-intent-model.md](execution-intent-model.md) —
  command-substitution intent facts used by the optimiser and shimmer.
- [compilation-unit-contracts.md](compilation-unit-contracts.md) —
  compilation unit orchestration and incremental cache expectations.

## Optimisation passes

- [tail-call-recursion-optimisation.md](tail-call-recursion-optimisation.md)
  — tail-call rewriting, recursion-to-loop, and accumulator hints
  (O121–O123).
- [optimiser-o124-unused-irule-procs.md](optimiser-o124-unused-irule-procs.md)
  — O124 comment out unused procs in iRules.
- [o125-code-sinking.md](o125-code-sinking.md) — O125 code sinking into
  decision blocks.

## Diagnostics and pass integration

- [pass-fact-ownership-matrix.md](pass-fact-ownership-matrix.md) —
  producer and consumer ownership map for core compiler facts.
- [downstream-pass-contracts.md](downstream-pass-contracts.md) — pass
  ownership, typed finding contracts, and overlap rules.
- [diagnostics-integration.md](diagnostics-integration.md) — aggregation
  and suppression policy boundary.
- [async-diagnostics-tiering.md](async-diagnostics-tiering.md) —
  fast/deep tiering and cancellation expectations.
- [phase4-lsp-consumers.md](phase4-lsp-consumers.md) — LSP feature
  consumers of shared compiler facts.

## Codegen boundary

- [bytecode-boundary.md](bytecode-boundary.md) — what stays in codegen
  and what should move earlier.
- [codegen-module-map.md](codegen-module-map.md) — package module map
  and ownership boundaries.
- [wasm-runtime-primitives.md](wasm-runtime-primitives.md) — Zig
  runtime primitives at the compiler-to-interpreter boundary
  (frame sync, namespace context, list element encoding, catch
  result separation, alias descriptors).

## Related KCS how-tos

- [How do I add a compiler pass?](../../kcs/kcs-howto-add-compiler-pass.md)
- [How do I debug an IR/CFG/SSA diagnostic?](../../kcs/kcs-howto-ir-cfg-ssa-diagnostics.md)
- [Stale compiler cache issue](../../kcs/kcs-issue-stale-compiler-cache.md)
- [Range drift issue](../../kcs/kcs-issue-range-drift.md)
- [Duplicate diagnostics issue](../../kcs/kcs-issue-duplicate-diagnostics.md)

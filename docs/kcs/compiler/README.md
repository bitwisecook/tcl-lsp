# Compiler KCS index

Focused, high-churn compiler guidance lives in this folder.

## Start here

- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md) — stage map and fact hand-off boundaries.
- [kcs-compiler-systems-overview.md](kcs-compiler-systems-overview.md) — subsystem contract map for quick ownership triage.
- [kcs-pass-authoring-checklist.md](kcs-pass-authoring-checklist.md) — quick checklist before/after pass changes.

## Pipeline stages

- [kcs-lexing-segmentation.md](kcs-lexing-segmentation.md) — token and command segmentation.
- [kcs-expression-parsing.md](kcs-expression-parsing.md) — Pratt parser, braced vs unbraced expressions.
- [kcs-cfg-construction.md](kcs-cfg-construction.md) — basic block decomposition patterns.
- [kcs-ssa-construction.md](kcs-ssa-construction.md) — version numbering and phi placement.
- [kcs-ir-types-lowering.md](kcs-ir-types-lowering.md) — IR node selection rules.
- [kcs-lowering-dispatch.md](kcs-lowering-dispatch.md) — arg_roles and command classification.
- [kcs-full-pipeline-walkthrough.md](kcs-full-pipeline-walkthrough.md) — end-to-end source to bytecode walkthrough.
- [kcs-control-flow-patterns.md](kcs-control-flow-patterns.md) — if/while/for/foreach/proc compilation.
- [kcs-error-recovery.md](kcs-error-recovery.md) — virtual token injection for malformed input.

## Analysis

- [kcs-sccp-core-analyses.md](kcs-sccp-core-analyses.md) — constant propagation and liveness.
- [kcs-constant-folding-type-inference.md](kcs-constant-folding-type-inference.md) — SCCP and type lattice.
- [kcs-taint-analysis.md](kcs-taint-analysis.md) — sources, sinks, colours, and propagation.
- [kcs-interprocedural-analysis.md](kcs-interprocedural-analysis.md) — ProcSummary construction.
- [kcs-optimisation-passes.md](kcs-optimisation-passes.md) — O100–O126 pass table and priorities.

## Infrastructure

- [kcs-command-registry.md](kcs-command-registry.md) — command metadata, specs, arity, and taint hints.
- [kcs-data-structure-reference.md](kcs-data-structure-reference.md) — pipeline types at each stage.
- [kcs-connection-scope.md](kcs-connection-scope.md) — cross-event variable flow in iRules.
- [kcs-dialects-events.md](kcs-dialects-events.md) — dialect filtering and event requirements.
- [kcs-namespace-resolution.md](kcs-namespace-resolution.md) — qualified name handling.
- [kcs-diagnostics-calculation.md](kcs-diagnostics-calculation.md) — two-phase diagnostic architecture.
- [kcs-codegen-internals.md](kcs-codegen-internals.md) — LVT, linearisation, labels, and peephole optimisation.

## Side-effects and effect classification

- [kcs-side-effects-system.md](kcs-side-effects-system.md) — structured side-effect hints, classification flow, and how to add hints to commands.

## Pipeline contracts

- [kcs-lowering-contracts.md](kcs-lowering-contracts.md) — lowering guarantees consumed by CFG/SSA/codegen.
- [kcs-cfg-ssa-fact-model.md](kcs-cfg-ssa-fact-model.md) — core fact model and consumption rules.
- [kcs-execution-intent-model.md](kcs-execution-intent-model.md) — command-substitution intent facts used by optimiser/shimmer.
- [kcs-compilation-unit-contracts.md](kcs-compilation-unit-contracts.md) — CU orchestration and incremental cache expectations.

## Optimisation passes

- [kcs-tail-call-recursion-optimisation.md](kcs-tail-call-recursion-optimisation.md) — tail-call rewriting, recursion-to-loop, and accumulator hints (O121–O123).

## Diagnostics and pass integration

- [kcs-pass-fact-ownership-matrix.md](kcs-pass-fact-ownership-matrix.md) — producer/consumer ownership map for core compiler facts.
- [kcs-downstream-pass-contracts.md](kcs-downstream-pass-contracts.md) — pass ownership, typed finding contracts, and overlap rules.
- [kcs-diagnostics-integration.md](kcs-diagnostics-integration.md) — aggregation and suppression policy boundary.
- [kcs-async-diagnostics-tiering.md](kcs-async-diagnostics-tiering.md) — fast/deep tiering and cancellation expectations.
- [kcs-phase4-lsp-consumers.md](kcs-phase4-lsp-consumers.md) — LSP feature consumers of shared compiler facts.

## Optimisation passes

- [kcs-optimiser-o124-unused-irule-procs.md](kcs-optimiser-o124-unused-irule-procs.md) — O124: comment out unused procs in iRules.
- [kcs-o125-code-sinking.md](kcs-o125-code-sinking.md) — O125 code sinking into decision blocks.

## Codegen boundary

- [kcs-bytecode-boundary.md](kcs-bytecode-boundary.md) — what stays in codegen vs what should move earlier.
- [kcs-codegen-module-map.md](kcs-codegen-module-map.md) — package module map and ownership boundaries.

## Troubleshooting and runbooks

- [kcs-troubleshooting-stale-cache.md](kcs-troubleshooting-stale-cache.md) — stale incremental cache triage.
- [kcs-troubleshooting-range-drift.md](kcs-troubleshooting-range-drift.md) — wrong-span/range drift triage.
- [kcs-troubleshooting-duplicate-diagnostics.md](kcs-troubleshooting-duplicate-diagnostics.md) — duplicate finding ownership triage.
- [kcs-contributor-ir-cfg-ssa-diagnostics-runbook.md](kcs-contributor-ir-cfg-ssa-diagnostics-runbook.md) — end-to-end contributor debug workflow.

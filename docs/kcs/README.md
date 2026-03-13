# KCS index

This folder contains small, searchable Knowledge-Centered Service (KCS) notes.

## Pipeline and analysis

- [kcs-lsp-feature-providers.md](kcs-lsp-feature-providers.md) — non-diagnostics LSP provider contracts and failure modes.
- [kcs-formatter-engine-contracts.md](kcs-formatter-engine-contracts.md) — formatter idempotency and rewrite contracts.
- [kcs-command-registry-event-model.md](kcs-command-registry-event-model.md) — command/event registry ownership and consumer rules.
- [kcs-workspace-indexing-contracts.md](kcs-workspace-indexing-contracts.md) — workspace cache/index/scanner contracts.
- [kcs-package-loading-contracts.md](kcs-package-loading-contracts.md) — package loading system: stdlib, tcllib, Tk, iRules cross-file procs.
- [kcs-lexing-contracts.md](kcs-lexing-contracts.md) — token/range fidelity rules for lexer changes.
- [kcs-parsing-contracts.md](kcs-parsing-contracts.md) — segmentation and recovery contracts.
- [kcs-core-lsp-shared-utility-contracts.md](kcs-core-lsp-shared-utility-contracts.md) — shared helper ownership and cross-feature consistency contracts.
- [kcs-lsp-diagnostics-publication.md](kcs-lsp-diagnostics-publication.md) — LSP diagnostics publication and suppression model.
- [kcs-vm-bytecode-test-boundary.md](kcs-vm-bytecode-test-boundary.md) — VM/bytecode identity and fixture boundary guidance.
- [kcs-vscode-extension-contracts.md](kcs-vscode-extension-contracts.md) — VS Code extension integration contracts.
- [kcs-pipeline-lsp-first.md](kcs-pipeline-lsp-first.md) — how to think about pipeline layering for LSP use.
- [kcs-shimmer-reference-behaviour.md](kcs-shimmer-reference-behaviour.md) — practical shimmer expectations and current validation strategy.
- [kcs-project-layout-contracts.md](kcs-project-layout-contracts.md) — repository layout ownership and dependency direction contracts.

## Fuzzing and test generation

- [kcs-differential-fuzzing-contracts.md](kcs-differential-fuzzing-contracts.md) — differential fuzzing oracle, bad-input corruption, and coverage-guided mutation contracts.
- [kcs-fuzz-finding-workflow.md](kcs-fuzz-finding-workflow.md) — how to triage, fix, test, and close fuzz findings.

## Scripts and test authoring

- [kcs-tcl-script-authoring-for-tests.md](kcs-tcl-script-authoring-for-tests.md) — patterns for writing Tcl examples for parser/analysis/bytecode tests.
- [kcs-irule-script-authoring-for-tests.md](kcs-irule-script-authoring-for-tests.md) — patterns for iRule examples focused on event flow and diagnostics.
- [kcs-screenshot-sample-authoring.md](kcs-screenshot-sample-authoring.md) — conventions for screenshot sample files and cursor marker comments.
- [kcs-irule-test-framework.md](kcs-irule-test-framework.md) — iRule Event Orchestrator: TMM simulation, command mocks, assertion DSL, Python bridge.

## Compiler architecture decomposition

- [compiler/README.md](compiler/README.md) — compiler-specific KCS landing page.

- [compiler/kcs-compiler-pipeline-overview.md](compiler/kcs-compiler-pipeline-overview.md) — stage map and hand-off boundaries.
- [compiler/kcs-compiler-systems-overview.md](compiler/kcs-compiler-systems-overview.md) — subsystem contract map for compiler ownership triage.
- [compiler/kcs-lowering-contracts.md](compiler/kcs-lowering-contracts.md) — lowering guarantees consumed by CFG/SSA.
- [compiler/kcs-cfg-ssa-fact-model.md](compiler/kcs-cfg-ssa-fact-model.md) — SSA/core fact production and consumption.
- [compiler/kcs-execution-intent-model.md](compiler/kcs-execution-intent-model.md) — execution-intent facts and command-substitution classification model.
- [compiler/kcs-side-effects-system.md](compiler/kcs-side-effects-system.md) — structured side-effect hints and effect classification flow.
- [compiler/kcs-codegen-module-map.md](compiler/kcs-codegen-module-map.md) — codegen package module map and ownership boundaries.
- [compiler/kcs-phase4-lsp-consumers.md](compiler/kcs-phase4-lsp-consumers.md) — how LSP features consume shared compilation-unit facts.
- [compiler/kcs-compilation-unit-contracts.md](compiler/kcs-compilation-unit-contracts.md) — CU orchestration and incremental cache contracts.
- [compiler/kcs-downstream-pass-contracts.md](compiler/kcs-downstream-pass-contracts.md) — pass ownership and typed finding contracts.
- [compiler/kcs-pass-fact-ownership-matrix.md](compiler/kcs-pass-fact-ownership-matrix.md) — pass -> fact -> consumer ownership matrix.
- [compiler/kcs-diagnostics-integration.md](compiler/kcs-diagnostics-integration.md) — aggregation, suppression, and policy boundary.
- [compiler/kcs-async-diagnostics-tiering.md](compiler/kcs-async-diagnostics-tiering.md) — tiered diagnostics scheduling and cancellation.
- [compiler/kcs-bytecode-boundary.md](compiler/kcs-bytecode-boundary.md) — what should stay in codegen vs move earlier.
- [compiler/kcs-pass-authoring-checklist.md](compiler/kcs-pass-authoring-checklist.md) — checklist for pass additions and updates.
- [compiler/kcs-optimiser-o124-unused-irule-procs.md](compiler/kcs-optimiser-o124-unused-irule-procs.md) — O124: comment out unused procs in iRules.
- [compiler/kcs-tail-call-recursion-optimisation.md](compiler/kcs-tail-call-recursion-optimisation.md) — O121–O123: tail-call and recursion optimisation passes.
- [compiler/kcs-troubleshooting-stale-cache.md](compiler/kcs-troubleshooting-stale-cache.md) — stale cache regression triage.
- [compiler/kcs-troubleshooting-range-drift.md](compiler/kcs-troubleshooting-range-drift.md) — range drift regression triage.
- [compiler/kcs-troubleshooting-duplicate-diagnostics.md](compiler/kcs-troubleshooting-duplicate-diagnostics.md) — duplicate diagnostics triage.
- [compiler/kcs-contributor-ir-cfg-ssa-diagnostics-runbook.md](compiler/kcs-contributor-ir-cfg-ssa-diagnostics-runbook.md) — IR->CFG->SSA->diagnostics runbook.
- [compiler/kcs-o125-code-sinking.md](compiler/kcs-o125-code-sinking.md) — O125 code sinking into decision blocks.

- [compiler/kcs-lexing-segmentation.md](compiler/kcs-lexing-segmentation.md) — token and command segmentation.
- [compiler/kcs-expression-parsing.md](compiler/kcs-expression-parsing.md) — Pratt parser, braced vs unbraced expressions.
- [compiler/kcs-cfg-construction.md](compiler/kcs-cfg-construction.md) — basic block decomposition patterns.
- [compiler/kcs-ssa-construction.md](compiler/kcs-ssa-construction.md) — version numbering and phi placement.
- [compiler/kcs-ir-types-lowering.md](compiler/kcs-ir-types-lowering.md) — IR node selection rules.
- [compiler/kcs-lowering-dispatch.md](compiler/kcs-lowering-dispatch.md) — arg_roles and command classification.
- [compiler/kcs-full-pipeline-walkthrough.md](compiler/kcs-full-pipeline-walkthrough.md) — end-to-end source to bytecode walkthrough.
- [compiler/kcs-control-flow-patterns.md](compiler/kcs-control-flow-patterns.md) — if/while/for/foreach/proc compilation.
- [compiler/kcs-error-recovery.md](compiler/kcs-error-recovery.md) — virtual token injection for malformed input.
- [compiler/kcs-sccp-core-analyses.md](compiler/kcs-sccp-core-analyses.md) — constant propagation and liveness.
- [compiler/kcs-constant-folding-type-inference.md](compiler/kcs-constant-folding-type-inference.md) — SCCP and type lattice.
- [compiler/kcs-taint-analysis.md](compiler/kcs-taint-analysis.md) — sources, sinks, colours, and propagation.
- [compiler/kcs-interprocedural-analysis.md](compiler/kcs-interprocedural-analysis.md) — ProcSummary construction.
- [compiler/kcs-optimisation-passes.md](compiler/kcs-optimisation-passes.md) — O100–O126 pass table and priorities.
- [compiler/kcs-command-registry.md](compiler/kcs-command-registry.md) — command metadata, specs, arity, and taint hints.
- [compiler/kcs-data-structure-reference.md](compiler/kcs-data-structure-reference.md) — pipeline types at each stage.
- [compiler/kcs-connection-scope.md](compiler/kcs-connection-scope.md) — cross-event variable flow in iRules.
- [compiler/kcs-dialects-events.md](compiler/kcs-dialects-events.md) — dialect filtering and event requirements.
- [compiler/kcs-namespace-resolution.md](compiler/kcs-namespace-resolution.md) — qualified name handling.
- [compiler/kcs-diagnostics-calculation.md](compiler/kcs-diagnostics-calculation.md) — two-phase diagnostic architecture.
- [compiler/kcs-codegen-internals.md](compiler/kcs-codegen-internals.md) — LVT, linearisation, labels, and peephole optimisation.

## User-facing features

- [features/README.md](features/README.md) — per-feature KCS docs used by the `help` command, MCP tool, and chat `/help`.
- [features/kcs-feature-tcl-verb-cli.md](features/kcs-feature-tcl-verb-cli.md) — unified verb-based Tcl zipapp contracts.

## KCS templates

- [templates/README.md](templates/README.md) — reusable templates for contract, troubleshooting, runbook, matrix, and reference KCS notes.

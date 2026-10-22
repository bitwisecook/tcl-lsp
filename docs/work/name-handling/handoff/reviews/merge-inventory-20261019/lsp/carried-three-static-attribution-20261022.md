# Three carried LSP selectors: static attribution before current execution

Prepared 2026-10-22. No current selector execution, stack measurement, product
fix or acceptance is claimed. Original current selectors, Tcl bytes, stack and
180-second budget remain unchanged.

## 80 namespace scopes

`refactor::inline_variable::tests::deeply_nested_namespaces_survive_scope_walk`
constructs80 nested namespace eval bodies, calls full Analyser::analyse under
tcl8.6, clones the complete AnalysisResult, then calls the cfg(test) scope walker.
The Core walker holds only scope/child references and a Vec of VarDef references;
its depth guard is256. The test's name does not identify the aborting phase.
AnalysisResult and Scope currently derive Clone; Scope children are Vec<Scope>,
so result cloning and final dropping are separate recursive candidates after
analysis. This is a static call-path observation, not a measured stack defect.

The genuine analyser path is initial -> document_command_realm (SourceCommand-
Bindings/IR source driver) -> retained inventories -> analyser body walk ->
parameter definitions -> diagnostic/CFG tail -> result/clear_run_state -> clone
-> Core walker -> drop. Existing debug source/diagnostic phase markers separate
the first stages without editing the fixture or changing stack allocation.

Only this selector is matched in the coordinator's exact origin/rust named-PASS
join. fcb7927 source blobs are not materialized in the active local object store;
no history fetch was performed. A different stale preload origin/rust ref is
not substituted for that baseline. The guarded capture records an actual abort,
but supplies no phase or current-frame attribution.

## 258 if bodies

`executable_regions::checked_potential_source_tests::checked_potential_source_retains_depth_exhaustion_as_incomplete`
constructs258 nested if {1} bodies and FIRST runs full Analyser::analyse.
Only afterward does visit_checked_analysis_executable_commands capture shared
SourceSyntaxCoverage, enumerate expected commands, and enter ExecutableWalker.
The walker retains source applicability/metadata through recursive region ->
original_source_regions -> original_region calls, with the unchanged256 limit.
Its large owned source-word/body projections are a possible retained-frame
candidate ONLY if the analyser and coverage stages have completed. No Core
walker cause is inferred before that boundary is observed.

## DB worker-dialect memo

Exact cached product source f6c4694f14c9be6f5f155397d30e46a6e72efc80 proves the
historical11927 unwrap is original_native_declaration_body_units.keys().next().
The unit and full-input unwraps precede it; the capsule unwrap follows it. The
old failure is therefore a missing declaration-only body key, not a measured
current capsule or complete-input failure. Raw ignored archive/log is absent in
the resumed trees, as reported by the coordinator.

The current selector ALREADY cites the immutable2026-10-19T14:20Z fresh-
interpreter decision and selects ir_module.procedures, requiring exactly one
installed body for proc sample {} {puts VALUE}. Tcl source/configuration and
None-versus-selected worker-dialect memo assertions remain. That authorized
body-owner correction must be named in current receipts; a current pass cannot
be called execution of the unchanged historical declaration-only assertion.

Current compilation_unit_for_config does pass Some(full input) and entry=None
through build_unit_with_keys_and_input. This is a static W210 source-entry
transport/equivalence seam reserved to compiler-names; it is not established as
the cause of this old test. Names confirmed no-path authoring must retain the
fresh/installed-proc semantics and explicit entries cannot become unknown.

## Exact execution and diagnostic order

Run /tmp/lsp-native-p2f/run.sh in the coordinator's Core/DB window first. Its
original exact selectors enforce unchanged stack,180 seconds, one test each,
source/provider/artifact guards and no target cleanup. No Cargo queue now.

If a Core selector fails, repeat that SAME actual artifact/selector separately
with TCL_LSP_TRACE_SOURCE_PHASES=1 and the same stack/budget. Mark this as a
separate diagnostic receipt, preserving the first original outcome.

| Last measured phase, if reproduced | Next bounded investigation |
| --- | --- |
| realm-start without realm-complete | Actual source driver/IR lowering call chain; inspect only that current path/frame before proposing driver storage changes. |
| walk-start without walk-complete | Namespace/if analyser recursion and owned per-body state, with current actual source purpose. |
| diagnostic phase before complete | CFG/SSA/optimiser build or diagnostic inventory; use existing detailed DIAGNOSTIC_PHASE markers. |
| analyser complete then abort (80) | Result clone, temporary/result drop, then the cfg(test) scope walk; current phase trace alone does not distinguish these. |
| analyser complete then abort (258) | Shared source coverage, Core source traversal and their retained bodies; investigate those exact owners before a storage-only patch. |

No current phase measured => no causal product patch is justified yet. The
historical CCG command/script frame sizes and Compiler lower_command29->18KiB
are source-pinned historical observations, not current80/258 frame receipts.
The parked24KiB framework is not reopened merely by these hypotheses. Do not
raise a stack/budget, lower a guard, clip the fixture or weaken completeness.

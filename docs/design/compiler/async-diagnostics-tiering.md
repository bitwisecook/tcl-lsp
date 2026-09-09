# Diagnostics tiering and cancellation

How a document's diagnostics are split into a fast tier and a deep tier, and
the currency rules that stop a slow pass from publishing results for an edit
the user has already moved past.

`run_diagnostics_analyser_path` (`rust/tcl-lsp-server/src/lib.rs`) runs the
deep pass — the per-file analyser walk, the compiler/optimiser checks, and
cross-file resolution, then the W120/W123 workspace refinement, the diagnostic
lifts, and one currency-guarded publish — as a single future, raced against
`DIAGNOSTICS_FAST_TIER_BUDGET` (40 ms):

- If the deep pass settles inside the budget, the client sees one publish.
- If it overruns, `publish_fast_tier` publishes the workspace-independent
  subset first — every analyser code no workspace pass can retract
  (`is_fast_tier`, i.e. `!DiagCode::refined_by_workspace()`, which excludes
  only W120 and W123) plus the pure source-style lints — and the deep pass
  replaces it for the same version when it finishes.
- Documents under `DIAGNOSTICS_FAST_TIER_MIN_LINES` (500) skip the race and
  take the single-publish path.

The fast tier is a strict subset of the deep tier: the deep pass only ever
removes W120/W123 and adds compiler, optimiser, and cross-file findings, so a
fast-tier diagnostic is never contradicted.

## Rules

1. **Currency.** `DeliveryCtx::deliver_fast_tier_if_current` and the deep
   publish both check the document revision atomically against edits and
   closes; a superseded run publishes nothing. The deep pass alone decides
   whether a version settled; a salsa cancellation makes the caller retry the
   document's latest state.
2. **Push only.** The fast tier never primes the pull-diagnostic cache and is
   skipped for a pull client — `textDocument/diagnostic` always serves the
   complete deep set.
3. **Shared suppression and finalisation.** Both tiers go through the same
   lifts and `finalise_diagnostics`, so `# noqa`, `# tcl-lsp: disable=`,
   disabled codes, tags, and severity overrides cannot differ between them.

## Failure modes

- A fast-tier diagnostic the deep pass would retract (a code missing from
  `refined_by_workspace`).
- A deep-tier panic dropping the whole deep set silently.
- Cancellation churn recomputing the deep pass on every keystroke.

## Anchors

- `rust/tcl-lsp-server/src/lib.rs` — `run_diagnostics_analyser_path`,
  `run_deep_diagnostics`, `publish_fast_tier`, `is_fast_tier`,
  `DeliveryCtx::deliver_fast_tier_if_current`, `finalise_diagnostics`
- `rust/tcl-core-types/src/diag_code.rs` — `DiagCode::refined_by_workspace`
- `rust/tcl-lsp-db/src/lib.rs` — `file_analysis`,
  `compiler_check_diagnostics`, `project_diagnostics` (the salsa queries the
  deep pass reads)
- `rust/tcl-lsp-server/tests/e2e/` — the diagnostic end-to-end suites

## See also

- [diagnostics-calculation.md](diagnostics-calculation.md)
- [diagnostics-integration.md](diagnostics-integration.md)
- [compiler architecture overview](architecture.md)

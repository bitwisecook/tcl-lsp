# Async diagnostics tiering and cancellation

How diagnostics are split into a fast tier and a deep background tier, and the
cancellation and version rules that stop a slow pass from publishing results
for an edit the user has already moved past.

Diagnostics are published in two tiers:

- Tier 1: fast parser/analyser/style results for immediate feedback.
- Tier 2: heavy compiler passes executed in background work and published incrementally.

This split is coordinated by `DiagnosticScheduler`, while `get_diagnostics()` provides unified aggregation contracts.

## Decision rules / contracts

1. **Fast-first publishing**
   - Tier 1 should avoid high-latency passes and return quickly after edits.
2. **Stale-work cancellation**
   - New document versions must cancel in-flight deep analysis before publish.
3. **Monotonic quality**
   - Deep-tier publish should enrich/replace diagnostics for the same document version, never regress to older snapshots.
4. **Shared suppression semantics**
   - Tier boundaries must not change `# noqa` and disabled-code behaviour.

## File-path anchors

- `rust/tcl-lsp-server/src/lib.rs` (`DiagnosticScheduler`)
- `rust/tcl-lsp-db/src/lib.rs` (`get_diagnostics`, phase-aware collection)
- `rust/tcl-lsp-db/src/lib.rs` (document version + CU cache interactions)

## Failure modes

- Stale deep-tier diagnostics published after a newer edit.
- Tier 2 pass exceptions dropping all deep diagnostics silently.
- Inconsistent suppression between quick and deep diagnostics.
- Excessive cancellation churn causing repeated heavy recomputation.

## Test anchors

- `rust/tcl-lsp-server/tests/e2e/` — the LSP diagnostic end-to-end suites.


## Discoverability

- [compiler KCS index](README.md)
- [compiler architecture overview](../../../docs/design/compiler-architecture.md)

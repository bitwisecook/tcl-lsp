# KCS: Async diagnostics tiering and cancellation

## Symptom

Editor diagnostics feel laggy or flicker because deep passes race with fresh edits and publish stale results.

## Operational context

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

- `lsp/async_diagnostics.py` (`DiagnosticScheduler`)
- `lsp/features/diagnostics.py` (`get_diagnostics`, phase-aware collection)
- `lsp/workspace/document_state.py` (document version + CU cache interactions)

## Failure modes

- Stale deep-tier diagnostics published after a newer edit.
- Tier 2 pass exceptions dropping all deep diagnostics silently.
- Inconsistent suppression between quick and deep diagnostics.
- Excessive cancellation churn causing repeated heavy recomputation.

## Test anchors

- `tests/test_diagnostic_phases.py`
- `tests/test_diagnostics.py`


## Discoverability

- [compiler KCS index](README.md)
- [compiler architecture overview](../../compiler-architecture.md)

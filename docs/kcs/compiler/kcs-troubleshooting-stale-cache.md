# KCS: Troubleshooting stale cache diagnostics

## Symptom

Diagnostics (or optimisation suggestions) appear to reference previous edits, incorrect source ranges, or old proc bodies after incremental changes.

## Operational context

Incremental analysis reuses per-procedure `FunctionUnit` and local interprocedural summaries across edits. This is coordinated by document-state cache management and `compile_source()` cache parameters.

## Decision rules / contracts

1. Cache keys must include stable proc identity and source-slice content hash.
2. Reused entries must be invalidated when procedure text or qualified identity changes.
3. Deep diagnostics should consume artefacts tied to the same document version.

## File-path anchors

- `core/compiler/compilation_unit.py`
- `lsp/workspace/document_state.py`
- `lsp/features/diagnostics.py`
- `lsp/async_diagnostics.py`

## Failure modes

- Proc cache reused for changed text due to key drift.
- Interproc local summary cache not pruned when call graph changed.
- Deep diagnostics publish from stale snapshot after a newer edit.

## Triage checklist

1. Reproduce with a proc rename + body edit sequence.
2. Inspect `DocumentState` cache merge/invalidate path.
3. Compare diagnostics output with and without cached CU reuse.
4. Confirm async cancellation of old deep tasks for new versions.

## Test anchors

- `tests/test_compilation_cache.py`
- `tests/test_diagnostics.py`
- `tests/test_diagnostic_phases.py`
- `tests/test_async_diagnostics.py`

## Discoverability

- [compiler KCS index](README.md)
- [compilation unit contracts](kcs-compilation-unit-contracts.md)
- [async diagnostics tiering](kcs-async-diagnostics-tiering.md)

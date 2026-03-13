# KCS: LSP diagnostics publication model

## Symptom

Editor diagnostics flicker, regress between edits, or differ from expected suppression/severity behaviour.

## Operational context

The LSP layer coordinates analysis output publication, including tiered scheduling and conversion to client-visible diagnostics.

## Decision rules / contracts

1. Publish fast baseline diagnostics first; enrich with deep results asynchronously.
2. Suppression and code-family policy must remain centralized and deterministic.
3. New LSP-facing diagnostic families must map cleanly to existing filtering controls.

## File-path anchors

- `lsp/features/diagnostics.py`
- `lsp/async_diagnostics.py`
- `lsp/server.py`

## Failure modes

- Stale deep diagnostics publishing after newer edits.
- Inconsistent suppression handling across analyser vs compiler-pass findings.
- Client-facing severity drift after adding new diagnostic families.

## Test anchors

- `tests/test_diagnostics.py`
- `tests/test_diagnostic_phases.py`
- `tests/test_async_diagnostics.py`

## Discoverability

- [KCS index](README.md)
- [compiler diagnostics integration](compiler/kcs-diagnostics-integration.md)
- [async tiering contracts](compiler/kcs-async-diagnostics-tiering.md)

# KCS: Workspace/indexing contracts

## Symptom

Cross-file navigation is stale or incomplete, symbol search misses expected items, or background scans/resolution lag behind edits.

## Operational context

Workspace services track per-document state, global proc indexes, scanning, and package resolution. LSP navigation and workspace symbols depend on this layer being fresh and deterministic.

## Decision rules / contracts

1. Document-state updates must preserve cache correctness across edits/errors.
2. Workspace index queries should tolerate partial/stale files conservatively.
3. Scanner and package resolver changes require cross-file navigation regression checks.

## File-path anchors

- `lsp/workspace/document_state.py`
- `lsp/workspace/workspace_index.py`
- `lsp/workspace/scanner.py`
- `core/packages/resolver.py`
- `lsp/features/workspace_symbols.py`

## Failure modes

- Proc index stale after rename/move causing wrong definition targets.
- Background scanner missing updates under burst edits.
- Package resolution false negatives masking available APIs.

## Test anchors

- `tests/test_workspace_index.py`
- `tests/test_workspace_symbols.py`
- `tests/test_scanner.py`
- `tests/test_package_resolver.py`
- `tests/test_definition.py`

## Discoverability

- [KCS index](README.md)
- [LSP feature providers](kcs-lsp-feature-providers.md)
- [stale cache troubleshooting](compiler/kcs-troubleshooting-stale-cache.md)

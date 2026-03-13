# KCS: feature — Workspace Symbols

## Summary

Search symbols across all open files in the workspace.

## Surface

lsp, all-editors

## How to use

- **Editor**: Ctrl+T and type a symbol name.
- **Settings**: Toggle with `tclLsp.features.workspaceSymbols`.

## Operational context

Searches the workspace index for procs, namespaces, and variables matching the query. Relies on the workspace scanner for cross-file indexing.

## File-path anchors

- `lsp/features/workspace_symbols.py`

## Failure modes

- Stale results if the workspace index is not refreshed.

## Test anchors

- `tests/test_workspace_symbols.py`

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)
- [Workspace indexing contracts](../kcs-workspace-indexing-contracts.md)

# KCS: feature — Document Symbols

## Summary

Outline of procs, namespaces, event handlers, and variables in the current file.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: Ctrl+Shift+O or the Outline panel.
- **MCP**: `symbols` tool — pass source code.
- **Settings**: Toggle with `tclLsp.features.documentSymbols`.

## Operational context

Produces a hierarchical symbol tree with procs nested inside namespaces, variables inside procs, and event handlers (iRules `when` blocks) at the top level.

## File-path anchors

- `lsp/features/document_symbols.py`

## Failure modes

- Symbols missing or mis-nested after parser changes.

## Test anchors

- `tests/test_document_symbols.py`

## Screenshots

- `17-document-symbols` — symbol picker showing proc outline

![symbol picker showing proc outline](../screenshots/17-document-symbols.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

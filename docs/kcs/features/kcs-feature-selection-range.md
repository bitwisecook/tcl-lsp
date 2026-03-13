# KCS: feature — Selection Range

## Summary

Smart expand/shrink selection by syntactic structure.

## Surface

lsp, all-editors

## How to use

- **Editor**: Shift+Alt+Right to expand selection, Shift+Alt+Left to shrink.
- **Settings**: Toggle with `tclLsp.features.selectionRange`.

## Operational context

Selection ranges are computed from the AST, expanding from the innermost expression outward through arguments, commands, blocks, procs, and namespaces.

## File-path anchors

- `lsp/features/selection_range.py`

## Failure modes

- Selection jumps over syntactic levels after AST changes.

## Test anchors

- `tests/test_selection_range.py`

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

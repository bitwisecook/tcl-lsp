# KCS: feature — Document Links

## Summary

Clickable links for URLs and file paths in comments and strings.

## Surface

lsp, all-editors

## How to use

- **Editor**: Ctrl+Click on a URL or file path to open it.
- **Settings**: Toggle with `tclLsp.features.documentLinks`.

## Operational context

The provider scans comments and string literals for URLs and file paths, making them clickable in the editor.

## File-path anchors

- `lsp/features/document_links.py`

## Failure modes

- Links not detected for unusual URL schemes.

## Test anchors

- `tests/test_document_links.py`

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

# KCS: feature — Inlay Hints

## Summary

Inline type and value information displayed alongside code.

## Surface

lsp, all-editors

## How to use

- **Editor**: Shown automatically as faded text inline with the code.
- **Settings**: Toggle with `tclLsp.features.inlayHints`.

## Operational context

Inlay hints show additional information such as parameter names and inferred types without modifying the source code.

## File-path anchors

- `lsp/features/inlay_hints.py`

## Failure modes

- Hints positioned incorrectly after document edits.

## Test anchors

- `tests/test_inlay_hints.py`

## Screenshots

- `21-inlay-hints` — inline hints alongside code

![inline hints alongside code](../screenshots/21-inlay-hints.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

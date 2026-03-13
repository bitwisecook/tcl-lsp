# KCS: feature — Folding

## Summary

Code folding for procs, namespaces, event handlers, and braced blocks.

## Surface

lsp, all-editors

## How to use

- **Editor**: Click fold markers in the gutter or use Ctrl+Shift+[ to fold.
- **Settings**: Toggle with `tclLsp.features.folding`.

## Operational context

Folding ranges are computed from the parsed AST, identifying proc bodies, namespace blocks, `when` event handlers, and multi-line braced expressions.

## File-path anchors

- `lsp/features/folding.py`

## Failure modes

- Folding ranges missing or incorrect after parser changes.

## Test anchors

- `tests/test_folding.py`

## Screenshots

- `20-folding` — code folded to show structure

![code folded to show structure](../screenshots/20-folding.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

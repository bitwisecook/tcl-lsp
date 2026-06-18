# KCS: feature — Inlay Hints

> **Audience:** User
> **Type:** Functionality

## Summary

Inline type and value information displayed alongside code.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Shown as faded text inline with the code when enabled.
- **Settings**: Inlay hints are split into two independent families, both
  off by default:
  - `tclLsp.features.inlayTypeHints` — inferred variable types (`: int`,
    `: str`) and format-string specifier labels.
  - `tclLsp.features.inlayParameterHints` — parameter-name labels at
    proc/method call sites (`NAME:`, `PROC_SCRIPT:`, …). These are more
    verbose and less likely to assist, so they are opt-in separately.

## Operational context

Inlay hints show additional information such as parameter names and inferred
types without modifying the source code. The two families map to the LSP
`InlayHintKind` values: type hints are `Type`, parameter-name hints are
`Parameter`. Enabling one does not enable the other.

## File-path anchors

- `server/features/inlay_hints.py`

## Failure modes

- Hints positioned incorrectly after document edits.

## Test anchors

- `tests/test_inlay_hints.py`

## Screenshots

- `21-inlay-hints` — inline hints alongside code

![inline hints alongside code](../screenshots/21-inlay-hints.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)

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
    proc/method call sites (`NAME:`, `PROC_SCRIPT:`, …).
- **Alias**: `tclLsp.features.inlayHints` sets the type hints only. An
  explicit `tclLsp.features.inlayTypeHints` wins when both are set.

## Operational context

Hints appear without changing the source. Type hints use the LSP `Type`
kind, parameter-name hints use `Parameter`. Enabling one does not enable
the other.

## Failure modes

- Hints positioned incorrectly after document edits.

## Screenshots

- `21-inlay-hints` — inline hints alongside code

![inline hints alongside code](../screenshots/21-inlay-hints.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)

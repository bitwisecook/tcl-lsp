# KCS: feature — Hover

## Summary

Command documentation, proc signatures, variable info, and taint status on hover.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: Hover over any symbol to see documentation.
- **MCP**: `hover` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.hover`.

## Operational context

The hover provider resolves the symbol under the cursor and returns documentation from the command registry, proc signatures from analysis, variable types, and taint tracking status for iRules.

## File-path anchors

- `lsp/features/hover.py`

## Failure modes

- Missing hover after command registry updates.
- Incorrect position mapping in multi-line constructs.

## Test anchors

- `tests/test_hover.py`

## Screenshots

- `02-hover-proc` — hover showing proc signature and documentation

![hover showing proc signature and documentation](../screenshots/02-hover-proc.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

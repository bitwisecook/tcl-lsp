# KCS: feature — Rename

## Summary

Rename procs and variables consistently across the file.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: F2 on a symbol, type the new name.
- **MCP**: `rename` tool — pass source, position, and new name.
- **Settings**: Toggle with `tclLsp.features.rename`.

## Operational context

Rename finds all references to the symbol and applies a consistent text edit. Uses shared proc-reference matching to ensure definition and all usages are updated.

## File-path anchors

- `lsp/features/rename.py`
- `core/analysis/proc_lookup.py`

## Failure modes

- Incomplete rename (misses some references).
- Rename applied to wrong symbol due to scope confusion.

## Test anchors

- `tests/test_rename.py`

## Screenshots

- `18-rename` — rename dialog inline

![rename dialog inline](../screenshots/18-rename.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

# KCS: feature — Find References

## Summary

Find all references to a proc or variable across the file.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: Shift+F12 or right-click > Find All References.
- **MCP**: `find_references` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.references`.

## Operational context

Locates all usages of the symbol under the cursor, including definitions, calls, and variable reads/writes. Uses shared proc-reference matching.

## File-path anchors

- `lsp/features/references.py`
- `core/analysis/proc_lookup.py`

## Failure modes

- Incomplete references after scope or namespace changes.

## Test anchors

- `tests/test_references.py`

## Screenshots

- `16-references` — find all references panel

![find all references panel](../screenshots/16-references.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

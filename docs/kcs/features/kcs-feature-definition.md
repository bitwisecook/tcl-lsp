# KCS: feature — Go to Definition

## Summary

Jump to proc or variable definitions within and across files.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: Ctrl+Click or F12 on a symbol.
- **MCP**: `goto_definition` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.definition`.

## Operational context

Resolves proc calls, variable references, namespace-qualified names, and BIG-IP cross-object references to their definition locations. Uses shared proc-reference matching from `core/analysis/proc_lookup.py`.

## File-path anchors

- `lsp/features/definition.py`
- `core/analysis/proc_lookup.py`

## Failure modes

- Definition not found after proc lookup or namespace resolution changes.

## Test anchors

- `tests/test_definition.py`

## Screenshots

- `15-definition` — peek definition inline

![peek definition inline](../screenshots/15-definition.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

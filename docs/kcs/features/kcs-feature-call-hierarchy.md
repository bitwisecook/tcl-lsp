# KCS: feature — Call Hierarchy

## Summary

View incoming and outgoing calls for a proc.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: Right-click a proc > Show Call Hierarchy, or Shift+Alt+H.
- **MCP**: `call_graph` tool — pass source for the full call graph.
- **Settings**: Toggle with `tclLsp.features.callHierarchy`.

## Operational context

The call hierarchy provider traces call relationships between procs, showing which procs call a given proc (incoming) and which procs it calls (outgoing).

## File-path anchors

- `lsp/features/call_hierarchy.py`
- `core/analysis/semantic_graph.py`

## Failure modes

- Missing edges when procs are called via variable indirection.

## Test anchors

- `tests/test_call_hierarchy.py`

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)

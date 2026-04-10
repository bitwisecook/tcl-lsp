# KCS: feature — Call Hierarchy

## Summary

View incoming and outgoing calls for a proc.

## Applies to

all-editors, MCP, analyser

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

## Example

Given this Tcl source:

```tcl
proc greet {name} {
    puts "Hello, [format_name $name]"
}

proc format_name {name} {
    return [string totitle $name]
}

greet "world"
```

Placing the cursor on `format_name` and running **Show Call
Hierarchy** opens a tree view where the **Incoming calls** pane
lists `greet` and the **Outgoing calls** pane lists `string` —
click either one to jump to its definition.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)

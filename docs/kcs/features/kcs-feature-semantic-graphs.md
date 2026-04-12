# KCS: feature — Semantic Graphs

> **Audience:** User
> **Type:** Functionality

## Summary

Structured call graph, symbol graph, and data-flow graph extraction from Tcl and iRules source code.

## Applies to

tcl-lsp CLI, MCP, Claude skill

## Question

How do I extract a call graph, symbol map, or data-flow report from my Tcl code?

## How to use

Three tools cover semantic graph extraction, each returning structured JSON:

| Tool | What it returns |
|------|----------------|
| `call_graph` | Proc nodes (name, params, line, purity), caller-to-callee edges with call sites, root and leaf procs. |
| `symbol_graph` | Scope hierarchy with nested namespaces, proc definitions, variable references, and `package require` dependencies. |
| `dataflow_graph` | Taint warnings, tainted variables, and per-proc effect annotations (pure, reads, writes, has barrier). |

### tcl-lsp CLI

```
tcl callgraph my_irule.tcl --json
tcl symbols my_irule.tcl --json
```

### MCP

```json
{"tool": "call_graph", "arguments": {"source": "proc a {} { b }\nproc b {} {}"}}
{"tool": "symbol_graph", "arguments": {"source": "..."}}
{"tool": "dataflow_graph", "arguments": {"source": "..."}}
```

### Claude Code

The `/irule-diagram` and `/irule-dataflow` skills wrap graph extraction with AI commentary.

## Example

A three-proc iRule produces a `call_graph` result like:

```json
{
  "nodes": [
    {"name": "::select_pool", "params": ["uri"], "pure": true},
    {"name": "::log_action", "params": ["msg"], "pure": false}
  ],
  "edges": [
    {"caller": "<top-level>", "callee": "::select_pool"},
    {"caller": "<top-level>", "callee": "::log_action"}
  ],
  "roots": ["<top-level>"],
  "leaf_procs": ["::select_pool", "::log_action"]
}
```

## Related

- [KCS feature index](README.md)
- [Call Hierarchy](kcs-feature-call-hierarchy.md) — the LSP provider for interactive call trees in the editor
- [Control-Flow Diagrams](kcs-feature-control-flow-diagrams.md) — Mermaid flowcharts from iRule event flow
- [MCP Server](kcs-feature-mcp-server.md)

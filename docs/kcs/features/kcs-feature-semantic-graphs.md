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
tcl symbolgraph my_irule.tcl --json
tcl dataflow my_irule.tcl --json
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

An iRule with two procs called from `HTTP_REQUEST` produces a `call_graph`
result like:

```json
{
  "nodes": [
    {"name": "::log_action", "params": ["msg"], "line": 3, "pure": false, "effects": "NONE"},
    {"name": "::select_pool", "params": ["uri"], "line": 0, "pure": true, "effects": "NONE"},
    {"name": "::when::HTTP_REQUEST", "params": [], "line": 6, "pure": false, "effects": "HTTP_STATE"}
  ],
  "edges": [
    {"caller": "::when::HTTP_REQUEST", "callee": "::log_action",
     "call_sites": [{"line": 8, "character": 4}]},
    {"caller": "::when::HTTP_REQUEST", "callee": "::select_pool",
     "call_sites": [{"line": 7, "character": 11}]}
  ],
  "roots": ["::when::HTTP_REQUEST"],
  "leaf_procs": ["::log_action", "::select_pool"]
}
```

## Related

- [KCS feature index](README.md)
- [Call Hierarchy](kcs-feature-call-hierarchy.md) — the LSP provider for interactive call trees in the editor
- [Control-Flow Diagrams](kcs-feature-control-flow-diagrams.md) — Mermaid flowcharts from iRule event flow
- [MCP Server](kcs-feature-mcp-server.md)

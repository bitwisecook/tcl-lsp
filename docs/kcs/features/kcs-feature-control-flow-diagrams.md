# KCS: feature — Control-Flow Diagrams

> **Audience:** User
> **Type:** Functionality

## Summary

Extract structured control-flow data from iRules for Mermaid diagrams, path enumeration, and test coverage analysis.

## Applies to

MCP, Claude skill

## Question

How do I generate a control-flow diagram or enumerate decision paths in my iRule?

## How to use

Two tools cover control-flow extraction:

| Tool | What it returns |
|------|----------------|
| `diagram` | JSON representation of event flow: events, if/switch decisions, terminal actions (pool, reject, redirect), and proc calls. Designed to feed a Mermaid flowchart or AI explanation. |
| `irule_cfg_paths` | Every unique path through the iRule to a terminal action, grouped by event, with conditions, path labels, and coverage hints. |

### MCP

```json
{"tool": "diagram", "arguments": {"source": "when HTTP_REQUEST { ... }"}}
{"tool": "irule_cfg_paths", "arguments": {"source": "when HTTP_REQUEST { ... }"}}
```

### Claude Code

The `/irule-diagram` skill wraps `diagram` and produces a rendered Mermaid flowchart with an explanation of the traffic flow.

## Example

A two-branch `HTTP_REQUEST` event produces a `diagram` result like:

```json
{
  "events": [{
    "name": "HTTP_REQUEST",
    "multiplicity": "per_request",
    "flow": [{
      "kind": "if",
      "condition": "[HTTP::method] eq \"GET\"",
      "branches": [
        {"condition": "true",  "body": [{"kind": "action", "command": "pool", "args": ["get_pool"]}]},
        {"condition": "else",  "body": [{"kind": "action", "command": "pool", "args": ["post_pool"]}]}
      ]
    }]
  }]
}
```

The AI turns this into a Mermaid diagram the reader can paste into any Markdown viewer.

## Related

- [KCS feature index](README.md)
- [Semantic Graphs](kcs-feature-semantic-graphs.md) — call graph, symbol graph, data-flow graph
- [Diagnostics](kcs-feature-diagnostics.md) — the analyser that powers the CFG extraction
- [MCP Server](kcs-feature-mcp-server.md)

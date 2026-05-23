# KCS: feature — MCP Server

## Summary

Model Context Protocol server exposing analysis/refactoring tools for AI agent integration.

## Applies to

MCP

## Availability

| Context | How |
|---------|-----|
| Zed Agent panel | Registered automatically as `tcl-lsp-mcp` context server |
| Claude Desktop | Add to `claude_desktop_config.json` |
| Any MCP client | `python -m ai.mcp.tcl_mcp_server` or `python tcl-lsp-mcp-server.pyz` |

## How to use

The MCP server communicates over stdio using JSON-RPC 2.0. Connect any MCP-compatible AI client and the tools become available:

| Tool | Description |
|------|-------------|
| `analyze` | Full analysis: diagnostics, symbols, events, event metadata |
| `validate` | Categorised validation report |
| `review` | Security-focused analysis |
| `convert` | Detect legacy patterns eligible for modernisation |
| `optimize` | Optimisation suggestions and rewritten source |
| `unminify_error` | Translate minified Tcl/iRule errors using a symbol map |
| `hover` | Hover information at a position |
| `complete` | Completions at a position |
| `goto_definition` | Find symbol definition |
| `find_references` | Find all symbol references |
| `symbols` | Document symbol hierarchy |
| `code_actions` | Quick fixes for a range |
| `format_source` | Format source code |
| `rename` | Rename a symbol |
| `event_info` | iRules event metadata |
| `command_info` | Command registry lookup |
| `event_order` | Events in firing order |
| `diagram` | Control flow diagram data |
| `call_graph` | Proc call graph |
| `symbol_graph` | Symbol relationship graph |
| `dataflow_graph` | Data-flow and taint graph |
| `xc_translate` | iRule to F5 XC translation |
| `tk_layout` | Tk widget tree extraction |
| `set_dialect` | Set active Tcl dialect |
| `help` | Feature catalogue |

## Operational context

Pure Python implementation — no heavy SDK, no pydantic, no C extensions. Runs from a zipapp (`tcl-lsp-mcp-server.pyz`). Uses the same analysis engine as the LSP server.

## File-path anchors

- `ai/mcp/tcl_mcp_server.py`
- `scripts/zipapp-main/mcp.py`

## Failure modes

- Python 3.10+ not available.
- Stdin/stdout hijacked by other tools.

## Test anchors

- `tests/test_mcp_server.py`

## Example

An AI agent calls the `analyze` tool over stdio using JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "analyze",
    "arguments": {
      "source": "when HTTP_REQUEST {\n  log local0. $unset_var\n}\n",
      "dialect": "f5-irules"
    }
  }
}
```

The server responds with a structured report listing the unresolved
variable diagnostic (`W211`), the single `HTTP_REQUEST` event, and
the full symbol table for the script. The agent can then follow up
with `code_actions` or `optimize` on the same source.

## Discoverability

- [KCS feature index](README.md)

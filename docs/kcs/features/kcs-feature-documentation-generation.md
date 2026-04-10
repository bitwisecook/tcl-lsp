# KCS: feature — Documentation Generation

> **Audience:** User
> **Type:** Functionality

## Summary

Generate docstrings for undocumented procs, extract structured proc metadata, and produce context packs for AI analysis.

## Applies to

VS Code, MCP, Claude skill

## Question

How do I auto-generate docstrings for my procs, or extract a structured summary of my code for AI tools?

## How to use

Three tools cover documentation generation:

| Tool | What it does |
|------|-------------|
| **Generate Docstring** | Adds docstring stubs to every undocumented proc in the file. |
| **Proc Docs** | Extracts structured metadata from every proc: name, parameters (with defaults), parsed docstring (`@param`, `@return`), and parameter traits. |
| **Context** | Produces a "context pack" summarising the file for AI consumption: dialect, diagnostics rollup, symbol inventory (events, procs, variables, namespaces), and event firing order. |

### VS Code

Run **Tcl: Generate Docstring for Proc** from the Command Palette.

### MCP

```json
{"tool": "update_docstrings", "arguments": {"source": "proc greet {name} { ... }"}}
{"tool": "read_proc_docs", "arguments": {"source": "..."}}
```

### Claude Code

The context pack is used internally by `/irule-create`, `/tcl-create`, and `/irule-review` to give the AI full awareness of the file before it generates or reviews code.

## Example

Running **Generate Docstring** on:

```tcl
proc greet {name} {
    puts "Hello, $name"
}
```

produces:

```tcl
# @brief <description>
# @param name <description>
proc greet {name} {
    puts "Hello, $name"
}
```

## Related

- [KCS feature index](README.md)
- [Hover](kcs-feature-hover.md) — displays the parsed docstring on hover
- [Signature Help](kcs-feature-signature-help.md) — uses `@param` annotations
- [MCP Server](kcs-feature-mcp-server.md)

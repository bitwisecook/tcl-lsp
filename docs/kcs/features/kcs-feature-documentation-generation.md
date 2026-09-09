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

| Tool | What it does |
|------|-------------|
| `generate_docstring` / `update_docstrings` | Adds a docstring stub to one proc, or to every undocumented proc in the file. |
| `read_proc_docs` | Extracts structured metadata from every proc: name, parameters (with defaults), parsed docstring (`@param`, `@return`), and parameter traits. |

### VS Code

Run **Tcl: Generate Docstring for Proc** from the Command Palette.

### MCP

```json
{"tool": "update_docstrings", "arguments": {"source": "proc greet {name} { ... }"}}
{"tool": "read_proc_docs", "arguments": {"source": "..."}}
```

### Claude Code

`/irule-create`, `/tcl-create`, and `/irule-review` build a context pack —
dialect, diagnostics rollup, symbol inventory, and event firing order — before
generating or reviewing code. It is not a tool you invoke yourself.

## Example

Running **Generate Docstring** on:

```tcl
proc greet {name} {
    puts "Hello, $name"
}
```

produces:

```tcl
# @brief TODO: describe greet
# @param name
proc greet {name} {
    puts "Hello, $name"
}
```

A parameter with a default carries `(default: …)`, and an `args` tail carries
`Additional arguments`.

## Related

- [KCS feature index](README.md)
- [Hover](kcs-feature-hover.md) — displays the parsed docstring on hover
- [Signature Help](kcs-feature-signature-help.md) — uses `@param` annotations
- [MCP Server](kcs-feature-mcp-server.md)

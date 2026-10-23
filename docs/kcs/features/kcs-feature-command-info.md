# KCS: feature — Command Info

> **Audience:** User
> **Type:** Functionality

## Summary

Look up metadata for any Tcl or iRules command: synopsis, switches, valid events, and dialect membership.

## Applies to

tcl-lsp CLI, MCP, Claude skill

## Question

How do I look up the synopsis, switches, and valid events for a Tcl or iRules command?

## How to use

### tcl-lsp CLI

```
tcl command-info --dialect f5-irules HTTP::uri
tcl command-info string --json
```

Pass the exact command name; `--dialect` selects the registry to look it up in
(default `tcl8.6`). Names are case-sensitive, and whitespace is part of the
name: `puts` and `::puts` find the built-in, while `Puts` and `" puts "`
report an unknown target. The result describes registry metadata and dialect
availability; it does not establish a live interpreter binding.
An explicitly supplied empty name is looked up as written; omitting the
argument is a CLI usage error.

### MCP

Call the `command_info` tool with the command name:

```json
{"tool": "command_info", "arguments": {"command_name": "HTTP::uri"}}
```

### Claude Code

The `/irule-event` skill provides command lookups as part of its event and command reference workflow.

## Example

```
$ tcl command-info --dialect f5-irules HTTP::uri
command: HTTP::uri
dialect: F5 iRules [f5-irules]
summary: Returns or sets the URI part of the HTTP request.
synopsis: HTTP::uri (URI)?
switches: -normalized
```

`--json` returns the same data as structured JSON for scripting.

## Related

- [KCS feature index](README.md)
- [Event Registry](kcs-feature-event-registry.md) — look up events instead of commands
- [MCP Server](kcs-feature-mcp-server.md) — the MCP surface that exposes this tool
- [Claude Code Skills](kcs-feature-claude-code-skills.md) — the `/irule-event` skill

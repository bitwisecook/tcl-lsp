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
tcl command-info HTTP::uri
tcl command-info "string length" --json
```

### MCP

Call the `command_info` tool with the command name:

```json
{"tool": "command_info", "arguments": {"command_name": "HTTP::uri"}}
```

### Claude Code

The `/irule-event` skill provides command lookups as part of its event and command reference workflow.

## Example

```
$ tcl command-info "HTTP::uri"
=== Command Info ===
  Command: HTTP::uri
  Summary: Returns or sets the URI part of the HTTP request.
  Synopsis: HTTP::uri (URI)?
  Switches: -normalized
  Valid in: HTTP_REQUEST, HTTP_RESPONSE, and 54 more events
```

The `--json` flag returns the same data as structured JSON for scripting.

## Related

- [KCS feature index](README.md)
- [Event Registry](kcs-feature-event-registry.md) — look up events instead of commands
- [MCP Server](kcs-feature-mcp-server.md) — the MCP surface that exposes this tool
- [Claude Code Skills](kcs-feature-claude-code-skills.md) — the `/irule-event` skill

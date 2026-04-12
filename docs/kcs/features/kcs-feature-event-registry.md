# KCS: feature — Event Registry

> **Audience:** User
> **Type:** Functionality

## Summary

Look up iRules event metadata (valid commands, priority, multiplicity, transport) and list events in firing order.

## Applies to

tcl-lsp CLI, MCP, Claude skill

## Question

How do I find out which commands are valid inside a given iRules event, and in what order events fire?

## How to use

Two commands cover the event registry:

- **event-info** — look up a single event by name.
- **event-order** — list every event in an iRule file in the order the traffic management microkernel fires them.

### tcl-lsp CLI

```
tcl event-info HTTP_REQUEST
tcl event-order /path/to/irule.tcl --json
```

### MCP

```json
{"tool": "event_info", "arguments": {"event_name": "HTTP_REQUEST"}}
{"tool": "event_order", "arguments": {"source": "when HTTP_REQUEST { ... }"}}
```

### Claude Code

The `/irule-event` skill wraps both lookups.

## Example

### event-info

```
$ tcl event-info HTTP_REQUEST
=== Event Info ===
  Event: HTTP_REQUEST
  Deprecated: no
  Multiplicity: per_request
  Side: client-side
  Transport: tcp
  Profiles: FASTHTTP, HTTP
  Valid commands: 1235
```

### event-order

```
$ tcl event-order my_irule.tcl
=== Event Firing Order (3 events) ===
  1. CLIENT_ACCEPTED  (per_connection)
  2. HTTP_REQUEST     (per_request)
  3. HTTP_RESPONSE    (per_request)
```

## Related

- [KCS feature index](README.md)
- [Command Info](kcs-feature-command-info.md) — look up commands instead of events
- [MCP Server](kcs-feature-mcp-server.md)
- [Claude Code Skills](kcs-feature-claude-code-skills.md)
- [iRule Skeleton](kcs-feature-irule-skeleton.md) — generates `when` blocks for selected events

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
f5 irule event-info HTTP_REQUEST
f5 irule event-order /path/to/irule.tcl --json
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
$ f5 irule event-info HTTP_REQUEST
event: HTTP_REQUEST
known: yes
lifecycle: available
multiplicity: per_request
description: Fires when request headers are fully parsed (pre-LB). On keep-alive connections, fires once per HTTP transaction. Pipeline: L7 iRules layer.
side: client-side
transport: tcp
profiles: FASTHTTP, HTTP
valid commands: 800
```

### event-order

```
$ f5 irule event-order my_irule.tcl
event order: 3 event(s)
  1. CLIENT_ACCEPTED (once_per_connection)
  2. HTTP_REQUEST (per_request)
  3. HTTP_RESPONSE (per_request)
```

## Related

- [KCS feature index](README.md)
- [Command Info](kcs-feature-command-info.md) — look up commands instead of events
- [MCP Server](kcs-feature-mcp-server.md)
- [Claude Code Skills](kcs-feature-claude-code-skills.md)
- [iRule Skeleton](kcs-feature-irule-skeleton.md) — generates `when` blocks for selected events

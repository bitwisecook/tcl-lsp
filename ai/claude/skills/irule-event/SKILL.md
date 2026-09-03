---
name: irule-event
description: "Look up iRules event or command reference from authoritative LSP registry metadata. Shows which commands are valid in an event, or which events support a command. Use when looking up iRules events, checking F5 iRule command availability, querying iRule event-command compatibility, or finding which events support a specific iRules command."
allowed-tools: mcp__tcl-lsp__event_info, mcp__tcl-lsp__command_info, Read
---

# iRule Event/Command Reference

## Steps

1. Read `../_prompts/irules_system.md`.
2. An event name (HTTP_REQUEST, CLIENT_ACCEPTED) → `mcp__tcl-lsp__event_info`
   with `event_name`; a command (HTTP::header, IP::client_addr) →
   `mcp__tcl-lsp__command_info` with `command_name`; unclear → ask. On an
   unknown name, report it and suggest similar names.
3. Present the registry metadata as facts, then clearly separated guidance.
   Events: when it fires, common commands, available request/response data,
   performance and safety notes, a minimal example. Commands: syntax and
   options, valid and invalid events, typical usage, common mistakes.

$ARGUMENTS

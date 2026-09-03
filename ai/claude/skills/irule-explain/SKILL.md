---
name: irule-explain
description: "Explain what an F5 iRule does by breaking down each event handler, describing data flow, noting security concerns, and summarising overall purpose. Uses LSP analysis for accurate context. Use when explaining iRule code, understanding what an iRule does, analysing iRule event handlers, or answering questions about F5 iRule behaviour."
allowed-tools: mcp__tcl-lsp__irule_with_context, Read
---

# iRule Explain

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. Call `mcp__tcl-lsp__irule_with_context` with the contents as
   `config_text`. If it fails (e.g. syntax errors), explain from the source
   alone and say LSP analysis was unavailable.
3. Explain: overall purpose; each event handler and when it fires; data flow
   between events (a variable set in CLIENT_ACCEPTED used in HTTP_REQUEST);
   security concerns the analyser raised. Focus on any specific question
   asked while keeping the full context.

## Output

A heading per event handler, code in ```tcl fences, security issues
prominent, and the event firing order with multiplicity (init /
once_per_connection / per_request).

$ARGUMENTS

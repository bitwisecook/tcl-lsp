---
name: irule-explain
description: "Explain what an F5 iRule does by breaking down each event handler, describing data flow, noting security concerns, and summarising overall purpose. Uses LSP analysis for accurate context. Use when explaining iRule code, understanding what an iRule does, analysing iRule event handlers, or answering questions about F5 iRule behaviour."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__irule_with_context, mcp__tcl-lsp__hud_chain, Read
---

# iRule Explain

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. A standalone iRule (`.tcl`, `when` blocks): call `mcp__tcl-lsp__analyze`
   with the contents as `source` for its events, firing order and
   diagnostics. A BIG-IP config (`ltm rule …`): call
   `mcp__tcl-lsp__irule_with_context` with `config_text` or `config_paths`
   (and `rule_path` for one rule) for the pools, data-groups and profiles it
   references, then `analyze` on the rule's `source`. If a tool fails (e.g.
   syntax errors), explain from the source alone and say LSP analysis was
   unavailable.
3. With a config, call `mcp__tcl-lsp__hud_chain` with `rule_path` for the
   HUD chain of every virtual server that attaches the rule: which profiles
   raise its events, and which other iRules run for the same events and share
   its connection variables. Without a config, say which profiles you assume.
4. Explain: overall purpose; each handler, when it fires and its priority;
   data flow between events (a variable set in CLIENT_ACCEPTED used in
   HTTP_REQUEST, values carried across keep-alive requests); procs and what
   they reach through `upvar`; security concerns the analyser raised. Focus
   on any specific question asked while keeping the full context.

## Output

A heading per event handler, code in ```tcl fences, security issues
prominent, and the event firing order with multiplicity (init /
once_per_connection / per_request). Note any other iRules on the same
virtual server that run for the same events.

$ARGUMENTS

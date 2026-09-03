---
name: irule-scaffold
description: "Generate an iRule skeleton from selected event names. Creates a template with proper log gating, local variable extraction, and placeholder sections for each event handler. Use when scaffolding new iRules, creating iRule templates, generating F5 iRule boilerplate, or starting a new iRule from event names."
allowed-tools: mcp__tcl-lsp__analyze, Read, Write
---

# iRule Scaffold

## Steps

1. Read `../_prompts/irules_system.md`.
2. Take the event names from the request. If none were given, list the
   common ones (RULE_INIT, CLIENT_ACCEPTED, HTTP_REQUEST, HTTP_RESPONSE,
   HTTP_REQUEST_DATA, HTTP_RESPONSE_DATA, SERVER_CONNECTED, CLIENT_CLOSED,
   SERVER_CLOSED, CLIENTSSL_HANDSHAKE, SERVERSSL_HANDSHAKE, DNS_REQUEST,
   DNS_RESPONSE, LB_SELECTED, LB_FAILED) with when each fires, and ask.
3. Generate the skeleton: with any HTTP event, a CLIENT_ACCEPTED block with
   `set debug 0` and `if {$debug}` around every log (never static:: for
   this); HTTP::uri / HTTP::host / HTTP::method extracted to locals at the
   top of HTTP events; commented placeholder sections
   (`# --- Request routing ---`); K&R braces, 4-space indent; a comment on
   when each event fires.
4. Write the file, then call `mcp__tcl-lsp__analyze` with the contents as
   `source`; fix and re-validate up to 3 iterations, then report the status
   and anything remaining.

$ARGUMENTS

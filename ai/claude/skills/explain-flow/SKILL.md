---
name: explain-flow
description: >
  Trace each session in a PCAP through a BIG-IP configuration and explain
  what the device did to it: matched virtual server, profiles, iRule HUD
  decisions (the `[HTTP::host]`, `[SSL::extensions]`, `[LB::server]`
  values the iRule branched on), event firing order, observed pool
  member and SNAT, termination analysis (TCP RST, F5 reset cause, TLS
  alert), and — with `simulate=true` — the actual outcome of running
  the iRule under c-tcl.  Understands `tcpdump -i <vlan>:np` captures
  that carry both front (client↔VIP) and back (TMM↔pool member) sides
  of the same conversation, paired via the F5 ethernet trailer.
  Decrypts TLS when given a NSS-format keylog file.  Use when a user
  asks "what would BIG-IP do with this capture?", "which iRule events
  fired for this flow?", "why did this connection reset?", or "explain
  this PCAP against this bigip.conf".
allowed-tools: mcp__tcl-lsp__explain_flow, Bash, Read
---

# Explain a flow against a BIG-IP config

## Steps

1. Read `../_prompts/explain_flow_system.md`: the compact JSON shape, how
   to narrate each field, the limitations, and when the verbose report is
   warranted.
2. Call `mcp__tcl-lsp__explain_flow` — preferred, it returns the pruned
   high-signal shape — with `pcap_path`, `config_paths` (array) or
   `config_text`, and as needed `use_tshark`, `keylog_path` (NSS keylog for
   HTTPS), `tshark_filter` (e.g. `"host 10.0.0.5 and port 443"`),
   `max_event_body_lines`, `simulate`.
3. Narrate each session per the guide.

## Keep the context small

- `tshark_filter` whenever the user asks about one flow; a big capture
  otherwise returns a session per 5-tuple.
- `max_event_body_lines` 4 when they want the decision, 0 to drop bodies.
- CLI fallback only when the MCP tool is unreachable, and then the **text**
  report: `f5-query explain-flow capture.pcap bigip.conf`
  (`--max-event-lines 8`, `--no-event-bodies`, `--tshark-filter '…'`).
  `--json` is the full report — for piping to another tool, never for your
  own reasoning.

$ARGUMENTS

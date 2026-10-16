---
name: explain-flow
description: >
  Explain what a BIG-IP configuration and a PCAP show about each session:
  matched virtual server, profiles, reconstructed source event sequence,
  connection-state annotations, observed pool member and SNAT, and termination
  analysis (TCP RST, F5 reset cause, TLS alert). With `simulate=true`, also
  reports a best-effort run of the iRule in the embedded Tcl VM with captured
  request state. Understands `tcpdump -i <vlan>:np` captures that carry both
  front (client↔VIP) and back (TMM↔pool member) sides of the same conversation,
  paired via the F5 ethernet trailer. Decrypts TLS when given a NSS-format
  keylog file. Use when a user asks what the capture and static analysis show
  for a BIG-IP configuration, which source events are relevant, or why a
  connection reset.
allowed-tools: mcp__tcl-lsp__explain_flow, Bash, Read
---

# Explain a flow against a BIG-IP config

## Steps

1. Read `../_prompts/explain_flow_system.md`: the JSON shape, how to
   narrate each field, and the limitations.
2. Call `mcp__tcl-lsp__explain_flow` — preferred — with `pcap_path`,
   `config_paths` (array) or `config_text`, and as needed `use_tshark`, `keylog_path` (NSS keylog for
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
  `--json` is the same value the MCP tool returns — for piping to another
  tool, never for your own reasoning.

$ARGUMENTS

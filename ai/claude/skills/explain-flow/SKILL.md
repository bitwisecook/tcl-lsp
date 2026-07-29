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
allowed-tools: Bash, Read
---

# Explain a flow against a BIG-IP config

This skill drives the `f5 explain-flow` verb / `explain_flow` MCP
tool to narrate every session in a packet capture against the BIG-IP
configuration that processed it.  Output is **pre-narrated and
context-pruned** by the Python side: the MCP tool returns a compact
JSON dict per session, not the raw per-flow dump.

## Steps

1. Read the shared output / narration guide from
   `ai/prompts/explain_flow_system.md` — it documents the compact
   JSON shape, how to narrate each field, the analyser's limitations,
   and when to fall back to the verbose `report_to_dict`.  The MCP
   `explain_flow` tool returns exactly that shape.
2. Invoke the `explain_flow` MCP tool (preferred) or the
   `f5 explain-flow` CLI fallback — see "How to invoke" below.
3. Narrate the per-session output to the user using the field-by-field
   guidance from the prompt file.

The Python that produces this output lives in
`dialects/f5/bigip/explain_flow.py` and is shared by both the MCP tool
(`ai/mcp/tcl_mcp_server.py`) and the CLI verb
(`tooling/f5/verbs/explain_flow.py`) — there is one source of truth.

## How to invoke (low-bloat, MCP-first)

**Prefer the MCP tool.**  It returns the same data the CLI text
report contains but already collapsed into the high-signal fields an
LLM needs — full Flow dicts, raw header maps, and untruncated event
bodies are dropped, empty fields omitted entirely.

```python
explain_flow(
    pcap_path="/path/to/flow.pcap",
    config_paths=["/path/to/bigip.conf"],  # or config_text=...
    use_tshark=True,  # add tshark enrichment
    keylog_path="/path/to/sslkeys.log",  # optional, for HTTPS
    tshark_filter="host 10.0.0.5 and port 443",  # scope to one flow
    max_event_body_lines=8,  # tighten to keep context small
    simulate=False,  # set True to run iRule under c-tcl
)
```

Tweak `max_event_body_lines` down (e.g. `4` or `0`) when the user
just wants the decision, not the iRule source.  Use `tshark_filter`
to scope huge captures to the one flow under investigation rather
than emitting a session per 5-tuple.

**Fall back to the CLI** only when the MCP tool isn't reachable.  In
that case prefer the **text** report — it's already operator-readable
and uses fewer tokens than JSON for the same information:

```bash
# Text report (recommended for context efficiency)
python3 -m tooling.f5.main explain-flow capture.pcap bigip.conf

# Tighten event bodies — equivalent of MCP max_event_body_lines
python3 -m tooling.f5.main explain-flow --max-event-lines 8 capture.pcap bigip.conf

# Just event names, no Tcl bodies at all
python3 -m tooling.f5.main explain-flow --no-event-bodies capture.pcap bigip.conf

# Scope to one flow via tshark display filter
python3 -m tooling.f5.main explain-flow \
    --tshark-filter 'host 10.0.0.5 and tcp.port == 443' \
    capture.pcap bigip.conf
```

`--json` exists but emits the *full* report (the verbose
`report_to_dict` shape) and will flood the LLM context.  Use `--json`
only when piping to another tool, never to feed back into your own
reasoning.

## Context-bloat controls (use them)

- Set `max_event_body_lines=4` when the user wants the decision and
  not the Tcl source.
- Set `max_event_body_lines=0` to drop event bodies entirely.
- Always pass `tshark_filter` when the user is asking about a
  specific flow ("port 443 to 10.0.0.5") — captures with thousands of
  sessions return one entry per 5-tuple otherwise.
- Don't request `--json` from the CLI for your own consumption —
  that's the verbose shape.  Use the MCP tool, or the text report.

$ARGUMENTS

---
name: explain-pcap
description: >
  Trace traffic in a PCAP through a BIG-IP configuration and explain what
  the device did to it: matched virtual server, ordered LTM profiles,
  attached LTM policies and APM, iRule event firing order with each
  `when EVENT { ... }` body, persistence, SNAT, pool & members, and GTM
  wide-IPs. Use when a user asks "what would BIG-IP do with this capture?",
  "which iRule events fired for this flow?", or "explain this PCAP against
  this bigip.conf". Provides enriching context for an LLM to narrate
  exactly what happened on the wire.
allowed-tools: Bash, Read
---

# Explain PCAP against a BIG-IP config

This skill drives the `f5 explain-pcap` verb (also exposed as the
`explain_pcap` MCP tool) to produce per-flow explanations: for every
unique 5-tuple in the capture, find the matching virtual server and
emit the resolved plan plus the event sequence the iRules would
execute on that flow.

## When to use

Use this skill when the user wants an explanation of what BIG-IP did to
real traffic.  Typical prompts:

- "Explain what happened to traffic in `capture.pcap` against this
  `bigip.conf`."
- "Which iRule events fire for this flow, and in what order?"
- "Show me the path through `/Common/rule_app` for the first HTTPS
  flow in the capture."
- "What profiles, SNAT, and pool members were involved?"

## Running

From the project root:

```bash
# Text report (default)
python3 -m explorer.f5_cli explain-pcap path/to/capture.pcap path/to/bigip.conf

# JSON for downstream LLM consumption
python3 -m explorer.f5_cli explain-pcap --json capture.pcap bigip.conf

# Use tshark for richer L7 decoding (HTTP method/Host/URI, TLS SNI)
python3 -m explorer.f5_cli explain-pcap --tshark capture.pcap bigip.conf

# Just the event names — skip the verbatim Tcl bodies
python3 -m explorer.f5_cli explain-pcap --no-event-bodies capture.pcap bigip.conf
```

Multiple config files may be passed; the first config containing a VS
whose `destination` matches a flow's IP:port wins for that flow.

Exit code is `0` if at least one flow matched a virtual server, `1`
if no flow matched any VS in the supplied configs.

## Output structure

For each flow:

1. **5-tuple summary** — src → dst, proto, packet count, TCP flags,
   TLS SNI/version (if observed), HTTP method/Host/URI (if observed).
2. **Matched virtual server** — full path, partition.
3. **Profiles** — in attach order, with type (`tcp`, `client_ssl`,
   `http`, …).
4. **LTM policies** attached to the VS.
5. **APM access profile** if present.
6. **GTM wide-IPs** parsed from the config.
7. **Expected iRule event firing order** — events present in the
   attached iRules, ordered by lifecycle and filtered to the events
   the observed traffic would actually trigger.
8. **iRule event bodies** — the verbatim `when EVENT { ... }` block
   for each fired event (truncated past `--max-event-lines`, default
   40, to keep LLM context manageable).
9. **Resolved plan** — the same view as `f5 explain virtual` for the
   matched VS (persistence, SNAT, default pool, pool members).

## How to reason about the output as an LLM

The output contains everything you need to narrate what happened:

- The **5-tuple summary** tells you what the client tried to do.
- The **profile list** tells you which of TCP/TLS/HTTP processing
  was enabled and in what order.
- The **event firing order** tells you which iRule hooks ran.
- The **event bodies** are the actual Tcl that executed; read each
  body and describe its effect (logging, header rewrites, pool
  selection, etc.).  The skill does *not* symbolically execute the
  Tcl — your job is to read it and explain.
- The **resolved plan** tells you which pool members the request
  could have reached and under what SNAT/persistence policy.

## Limitations

- The static event ordering is driven by attached profiles and
  observed L7 features.  It does not honour `event disable` or
  conditional `when ... { return }` patterns at runtime.
- Path-through-iRule analysis is *static*: the skill surfaces the
  full `when` block bodies; it does not follow `if`/`switch`
  branches based on payload bytes.
- LTM policy bodies, GTM probe results, and APM session state are
  not stored in the parsed config (only headers); those are listed
  by name only.

## MCP equivalent

The same logic is also exposed via the `explain_pcap` tool of the
tcl-lsp MCP server.  Pass `pcap_path` plus either `config_text` or
`config_paths`; the JSON shape mirrors the CLI's `--json` output.

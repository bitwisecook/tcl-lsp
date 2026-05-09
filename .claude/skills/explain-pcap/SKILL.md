---
name: explain-pcap
description: >
  Trace each session in a PCAP through a BIG-IP configuration and explain
  what the device did to it: matched virtual server, ordered LTM profiles,
  attached LTM policies and APM, iRule event firing order with each
  `when EVENT { ... }` body, persistence, SNAT, observed pool member and
  SNAT IP, GTM wide-IPs, and a termination analysis (TCP RST, F5 reset
  cause, TLS alert).  Understands `tcpdump -i <vlan>:np` captures that
  carry both the front (client↔VIP) and back (TMM↔pool member) sides of
  the same conversation, paired via the F5 ethernet trailer.  Optionally
  decrypts TLS using a NSS-format keylog file.  Use when a user asks
  "what would BIG-IP do with this capture?", "which iRule events fired
  for this flow?", "why did this connection reset?", or "explain this
  PCAP against this bigip.conf".  Provides enriching context for an LLM
  to narrate exactly what happened on the wire.
allowed-tools: Bash, Read
---

# Explain PCAP against a BIG-IP config

This skill drives the `f5 explain-pcap` verb (also exposed as the
`explain_pcap` MCP tool) to produce per-session explanations: it pairs
flows into bidirectional connections, then pairs front-side and
back-side connections into one logical session via the F5 ethernet
trailer's peer-tuple, finds the matching virtual server, and emits the
resolved plan plus the event sequence the iRules would execute.

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

# Use tshark for richer L7 decoding (HTTP/TLS, F5 reset cause)
python3 -m explorer.f5_cli explain-pcap --tshark capture.pcap bigip.conf

# Decrypt TLS using a NSS-format keylog file (implies --tshark)
python3 -m explorer.f5_cli explain-pcap --keylog ssl.log capture.pcap bigip.conf

# Just the event names — skip the verbatim Tcl bodies
python3 -m explorer.f5_cli explain-pcap --no-event-bodies capture.pcap bigip.conf
```

Multiple config files may be passed; the first config containing a VS
whose `destination` matches a flow's IP:port wins for that flow.

Exit code is `0` if at least one flow matched a virtual server, `1`
if no flow matched any VS in the supplied configs.

## Output structure

For each session:

1. **Front summary** — client↔VIP 5-tuple, packet counts each
   direction, TCP flags, TLS SNI/version/cipher/ALPN (if observed or
   decrypted via keylog), HTTP method/Host/URI, HTTP response code,
   TLS alert (if any), TCP RST count, F5 reset-cause text.
2. **Back summary** — TMM↔pool-member 5-tuple, same shape as front.
   Present only when the capture was taken with `tcpdump -i <vlan>:np`
   so the F5 trailer's HIGH TLV pairs the two sides.
3. **Matched virtual server** — full path, partition.
4. **Pool member chosen (observed)** — actual `address:port` the back
   side reached, derived from the back-side flow's destination.
5. **SNAT applied (observed)** — actual `address:port` TMM used as
   the source on the back side, when it differs from the client.
6. **Profiles** — in attach order, with type (`tcp`, `client_ssl`,
   `http`, …).
7. **LTM policies** attached to the VS.
8. **APM access profile** if present.
9. **GTM wide-IPs** parsed from the config.
10. **Expected iRule event firing order** — events present in the
    attached iRules, ordered by lifecycle and filtered to the events
    the observed traffic would actually trigger (TLS ClientHello →
    `CLIENTSSL_*`, HTTP request → `HTTP_REQUEST`, HTTP response →
    `HTTP_RESPONSE`, RST/FIN → `*_CLOSED`).
11. **iRule event bodies** — the verbatim `when EVENT { ... }` block
    for each fired event (truncated past `--max-event-lines`, default
    40, to keep LLM context manageable).
12. **Termination analysis** — narrative of why the connection ended:
    which side reset, after how many bytes, F5 reset-cause text from
    the trailer (LOW/MED TLV) when present, TLS alert text, or
    "graceful FIN teardown" when no RST occurred.
13. **Resolved plan** — the same view as `f5 explain virtual` for the
    matched VS (persistence, SNAT, default pool, pool members).

## How to reason about the output as an LLM

The output contains everything you need to narrate what happened:

- The **front summary** tells you what the client tried to do.  The
  **back summary** (when present) tells you what TMM did to the pool
  member after the L7 work — observed pool member and SNAT IP let
  you confirm load balancing and source-NAT actually executed as
  configured.
- The **profile list** tells you which of TCP/TLS/HTTP processing
  was enabled and in what order.
- The **event firing order** tells you which iRule hooks ran.
- The **event bodies** are the actual Tcl that executed; read each
  body and describe its effect (logging, header rewrites, pool
  selection, etc.).  The skill does *not* symbolically execute the
  Tcl — your job is to read it and explain.
- The **resolved plan** tells you which pool members the request
  could have reached and under what SNAT/persistence policy.
- The **termination analysis** tells you whether the session ended
  cleanly or via RST, which side reset, and what the F5 reset cause
  was.  Combine this with TLS alerts and HTTP response codes to
  explain *why*: e.g. "client sent RST after server replied 500" or
  "BIG-IP reset because the F5 trailer reports 'POOL_DOWN — no usable
  pool member'".

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

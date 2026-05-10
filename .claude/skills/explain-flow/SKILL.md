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

## How to invoke (low-bloat, MCP-first)

**Prefer the MCP tool.**  It returns the same data the CLI text
report contains but already collapsed into the high-signal fields an
LLM needs — full Flow dicts, raw header maps, and untruncated event
bodies are dropped, empty fields omitted entirely.

```python
explain_flow(
    pcap_path="/path/to/flow.pcap",
    config_paths=["/path/to/bigip.conf"],   # or config_text=...
    use_tshark=True,                         # add tshark enrichment
    keylog_path="/path/to/sslkeys.log",      # optional, for HTTPS
    tshark_filter="host 10.0.0.5 and port 443",  # scope to one flow
    max_event_body_lines=8,                  # tighten to keep context small
    simulate=False,                          # set True to run iRule under c-tcl
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
python3 -m explorer.f5_cli explain-flow capture.pcap bigip.conf

# Tighten event bodies — equivalent of MCP max_event_body_lines
python3 -m explorer.f5_cli explain-flow --max-event-lines 8 capture.pcap bigip.conf

# Just event names, no Tcl bodies at all
python3 -m explorer.f5_cli explain-flow --no-event-bodies capture.pcap bigip.conf

# Scope to one flow via tshark display filter
python3 -m explorer.f5_cli explain-flow \
    --tshark-filter 'host 10.0.0.5 and tcp.port == 443' \
    capture.pcap bigip.conf
```

`--json` exists but emits the *full* report (the verbose
`report_to_dict` shape) and will flood the LLM context.  Use `--json`
only when piping to another tool, never to feed back into your own
reasoning.

## What the compact MCP shape looks like

The MCP tool returns a JSON dict with fixed top-level keys:

```jsonc
{
  "pcap": "/path/to/flow.pcap",
  "matched": 1,                    // # sessions that matched a VS
  "sessions_total": 3,             // total sessions, incl. unmatched
  "tshark": true,
  "keylog": "...",                 // present if used
  "tshark_filter": "...",          // present if used
  "gtm_wide_ips_in_config": [...], // GLOBAL inventory, not per-session
  "sessions": [ /* one entry per session, see below */ ]
}
```

Each session is the high-signal subset — empty fields are omitted:

```jsonc
{
  "summary": "1.2.3.4:11111 → /partition/vs_app | SNI=api.example.com | GET /v1/health → 200 | pool→ 10.0.0.10:8080 | snat→ 10.0.0.5:22222",
  "matched_vs": "/partition/vs_app",
  "flow": { "client": "1.2.3.4:11111", "vip": "5.6.7.8:443",
            "pool_member": "10.0.0.10:8080", "snat": "10.0.0.5:22222",
            "proto": "tcp" },
  "captured_request": { "method": "GET", "host": "api.example.com",
                         "uri": "/v1/health", "tls_sni": "api.example.com",
                         "tls_version": "TLS1.3", "tls_cipher": "..." },
  "captured_response": { "status": "200" },
  "profiles": ["tcp (lab_tcp)", "client_ssl (lab_clientssl_valid)", "http (lab_http)"],
  "ltm_policies": ["/partition/lab_policy_rewrite"],
  "policy_decisions": [
    { "policy": "/partition/lab_policy_rewrite", "strategy": "first-match",
      "fired": [
        { "rule": "api_route", "ordinal": 1,
          "matched_on": [
            { "field": "http-host.host", "operator": "equals",
              "expected": ["api.example.com"], "actual": "api.example.com" },
            { "field": "http-uri.path", "operator": "starts-with",
              "expected": ["/v1/"], "actual": "/v1/health" }
          ],
          "actions": [
            { "target": "forward", "verb": "select",
              "value": "/partition/lab_pool_api" }
          ] }
      ] }
  ],
  "events_fired": ["RULE_INIT", "CLIENT_ACCEPTED", "CLIENTSSL_CLIENTHELLO",
                    "HTTP_REQUEST", "LB_SELECTED", "SERVER_CONNECTED"],
  "irule_decisions": [
    { "event": "CLIENTSSL_CLIENTHELLO", "command": "SSL::extensions",
      "value": "sni=api.example.com, alpn=h2" },
    { "event": "HTTP_REQUEST", "command": "HTTP::host",
      "value": "api.example.com" }
  ],
  "irule_bodies": [
    { "rule": "rule_sni", "event": "CLIENTSSL_CLIENTHELLO",
      "body": "if { [SSL::extensions exists -type 0] } { ... }\n... (truncated)" }
  ],
  "termination": "graceful FIN teardown (no RST)",
  "simulated": { "pool": "/partition/lab_pool_api",
                 "decisions": [ { "category": "lb", "action": "pool_select",
                                   "value": "/partition/lab_pool_api" } ] }
}
```

## How to narrate the output

The fields are designed so you can build the narrative directly:

1. **Read `summary`** — that's the one-line gist; quote it to the user.
2. **`captured_request` + `captured_response`** — what the client sent
   and what came back (or, if the connection didn't get that far,
   what was missing).
3. **`profiles`** — which BIG-IP code paths ran (TCP-only,
   TLS-decrypt, HTTP-aware, etc.).  Profile order is attach order.
4. **`events_fired` + `irule_decisions`** — answer "which iRule
   branches were taken and what state did they read?".  Each entry in
   `irule_decisions` says "in event X, the iRule looked at command Y
   and saw value Z" — exactly what you need to explain a
   `[HTTP::host] equals "..."` branch or an
   `[SSL::extensions servername]` route.
5. **`irule_bodies`** — only consult these when you need to quote
   Tcl.  They're already truncated; don't ask for
   `max_event_body_lines>20` unless absolutely needed.
5a. **`policy_decisions`** — answers "which LTM policy rule fired and
   what did it do?".  Each entry shows the rule name, the conditions
   that matched (with the actual captured value beside the expected
   one), and the action(s) it produced.  Only **fired** rules are
   listed in the compact shape; an empty `fired` array means the
   policy was evaluated but no rule matched (every rule had at least
   one condition that didn't match, or the policy had no rules at
   all).  Note that a rule with zero conditions is treated as
   unconditional and will always match — so if a "default" rule is
   defined, it will be in `fired` for `first-match`/`all-match`
   strategies whenever no earlier rule won.  Compact entries only
   include the conditions that actually matched (`matched_on`); for
   the full per-condition trace including non-matched and
   unevaluable conditions, fall back to the verbose
   `report_to_dict`.  Limited to the medium-scope operand set
   (host / uri / method / header / SNI / tcp address) and action set
   (forward / redirect / replace / header insert+remove / reset).
   When a policy uses `best-match`, the reported `strategy` is
   `best-match-approx` because we approximate F5's true best-match
   as "rule with the most conditions wins" — operand-specificity
   weighting isn't reproduced.
6. **`flow.pool_member` + `flow.snat`** — observed values.  If
   `simulated.pool` differs from `flow.pool_member`, mention it
   ("the iRule would route to X, but the captured back-side went to
   Y" — that gap usually means the capture pre-dates the rule edit
   or another rule overrode the choice).
7. **`termination`** — explain *why* the session ended.  RST + an F5
   reset cause string (e.g. "POOL_DOWN") gives a definitive answer.
8. **`simulated`** is the truth source when present — it's the
   actual outcome from running the iRule under c-tcl with the
   captured state.  If `simulated.error` is set, fall back to the
   static analysis above.

## Context-bloat controls (use them)

- Set `max_event_body_lines=4` when the user wants the decision and
  not the Tcl source.
- Set `max_event_body_lines=0` to drop event bodies entirely.
- Always pass `tshark_filter` when the user is asking about a
  specific flow ("port 443 to 10.0.0.5") — captures with thousands of
  sessions return one entry per 5-tuple otherwise.
- Don't request `--json` from the CLI for your own consumption —
  that's the verbose shape.  Use the MCP tool, or the text report.

## Limitations

- `tshark_filter` runs the entire extraction through tshark; F5
  trailer peer-tuples (used to pair `:np` front/back sides) are only
  populated by the built-in walker, so a filtered run loses
  front/back session pairing.  Drop the filter when investigating
  proxied traffic.
- `simulated` requires `tclsh` on PATH and one orchestrator
  subprocess per matched session — slower; avoid on captures with
  hundreds of sessions unless `tshark_filter` is also set.
- The static event ordering is driven by attached profiles and
  observed L7 features.  It does not honour `event disable` or
  conditional `when ... { return }` patterns at runtime; consult
  `simulated` for runtime ground truth.
- Path-through-iRule analysis is *static* unless `simulate=true`: the
  skill surfaces the relevant `when` block bodies; it does not follow
  `if`/`switch` branches based on payload bytes by itself.
- GTM probe results / APM session state aren't retained in the parsed
  config (only headers).  `gtm_wide_ips_in_config` is therefore a
  *global* inventory at the report level, not a per-session list —
  annotate accordingly when citing it.
- LTM policy evaluation covers a medium operand surface only:
  `http-host`, `http-uri` (host/path/query), `http-method`,
  `http-header` (with named target), `ssl-extension server-name`
  (SNI), and `tcp address`.  Actions covered: `forward select`,
  `http-reply redirect`, `http-uri replace`, `http-header
  insert/remove`, `tcp reset`.  Operands outside this set (cookie,
  geoip, ssl-cert, rate-limit) parse but evaluate as no-match with a
  `note` explaining why.  `best-match` strategy is approximated as
  "rule with the most conditions wins" — F5's true algorithm weights
  operand specificity which we don't reproduce.

## When to use the verbose `report_to_dict` shape instead

Only when:

- The user wants every captured packet's full 5-tuple + flag set,
  not just matched sessions.
- They're piping the output to another tool that expects the full
  per-flow Flow dicts.
- They explicitly asked for "the raw JSON" / "everything".

In all other cases the compact MCP shape is the right answer — it's
designed to give you the operator-relevant fields with the smallest
possible token cost.

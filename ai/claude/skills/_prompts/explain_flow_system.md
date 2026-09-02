# `explain_flow` — output shape and narration guide

Shared reference for the BIG-IP `explain_flow` analyser, loaded by the
`explain_flow` MCP tool (`rust/tcl-mcp/src/bigip.rs`) and the Claude
`explain-flow` skill. Both sit over the `f5 explain-flow` verb
(`rust/f5-cli/src/commands/explain_flow.rs`, reached via
`f5_cli::explain_flow_value`); keep this file in sync with the shape that
function returns — it is the contract the LLM consumes.

## The compact MCP shape

Fixed top-level keys:

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

Each session is the high-signal subset; empty fields are omitted:

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

## Narrating a session

1. Quote `summary` — the one-line gist.
2. `captured_request` + `captured_response`: what the client sent and what
   came back, or how far the connection got.
3. `profiles`: which BIG-IP code paths ran (TCP-only, TLS-decrypt,
   HTTP-aware); order is attach order.
4. `events_fired` + `irule_decisions`: which branches were taken and what
   they read — each decision is "in event X the iRule looked at command Y
   and saw Z", which explains a `[HTTP::host] equals` branch or an SNI route.
5. `policy_decisions`: which LTM policy rule fired and what it did; only
   fired rules appear, with the conditions that matched (`matched_on`,
   expected beside actual) and the actions. An empty `fired` means the
   policy ran and nothing matched; a rule with zero conditions always
   matches, so a "default" rule shows in `fired` under first-match /
   all-match whenever no earlier rule won. `best-match` is reported as
   `best-match-approx` ("most conditions wins"; F5's operand-specificity
   weighting is not reproduced). The full per-condition trace, including
   non-matched and unevaluable conditions, is only in the verbose shape.
6. `irule_bodies`: consult only when you need to quote Tcl; already
   truncated — do not ask for `max_event_body_lines > 20` without cause.
7. `flow.pool_member` + `flow.snat` are observed. If `simulated.pool` differs,
   say so: the capture usually pre-dates the rule edit or another rule
   overrode the choice.
8. `termination` explains why the session ended; an RST with an F5 reset
   cause (e.g. `POOL_DOWN`) is definitive.
9. `simulated`, when present, is the truth source (the iRule run under
   c-tcl with the captured state); on `simulated.error` fall back to the
   static analysis.

## Limitations

- `tshark_filter` routes extraction through tshark; the F5 trailer
  peer-tuples that pair `:np` front/back sides come only from the built-in
  walker, so a filtered run loses front/back pairing. Drop the filter for
  proxied traffic.
- `simulated` needs `tclsh` on PATH and one orchestrator subprocess per
  matched session; avoid on captures with hundreds of sessions unless
  `tshark_filter` is set.
- Static event ordering comes from attached profiles and observed L7
  features; it does not honour `event disable` or conditional
  `when ... { return }` — consult `simulated` for runtime truth.
- Path-through-iRule analysis is static unless `simulate=true`: it surfaces
  the relevant `when` bodies, it does not follow branches on payload bytes.
- GTM probe results and APM session state are not retained;
  `gtm_wide_ips_in_config` is a report-level global inventory, not
  per-session.
- LTM policy evaluation covers operands `http-host`, `http-uri`
  (host/path/query), `http-method`, `http-header` (named), `ssl-extension
  server-name`, `tcp address`, and actions `forward select`, `http-reply
  redirect`, `http-uri replace`, `http-header insert/remove`, `tcp reset`.
  Other operands (cookie, geoip, ssl-cert, rate-limit) parse but evaluate
  as no-match with a `note`.

## When to use the verbose `report_to_dict` shape

Only when the user wants every packet's full 5-tuple and flags, is piping to
a tool that expects the full per-flow dicts, or explicitly asks for the raw
JSON. Otherwise the compact shape is the answer — it carries the
operator-relevant fields at the smallest token cost.

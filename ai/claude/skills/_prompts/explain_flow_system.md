# `explain_flow` — output shape and narration guide

Shared reference for the BIG-IP `explain_flow` analyser, loaded by the
`explain_flow` MCP tool (`rust/tcl-mcp/src/bigip.rs`) and the Claude
`explain-flow` skill. Both sit over the `f5 explain-flow` verb
(`rust/f5-cli/src/commands/explain_flow.rs`, reached via
`f5_cli::explain_flow_value`); keep this file in sync with the shape that
function returns — it is the contract the LLM consumes.

## The report shape

The MCP tool and `f5 explain-flow --json` return the same value. Fixed
top-level keys, all always present:

```jsonc
{
  "pcap_path": "/path/to/flow.pcap",
  "flow_count": 6,                 // one-directional flows walked
  "session_count": 3,              // sessions, incl. unmatched
  "matched_count": 1,              // # sessions that matched a VS
  "used_tshark": true,
  "keylog_path": "",               // "" when unused
  "tshark_filter": "",             // "" when unused
  "sessions": [ /* one entry per session, see below */ ]
}
```

Each session carries every key; an unset field is `""`, `[]`, `false`, or
`null` rather than being omitted:

```jsonc
{
  "session": {
    // Front is client↔VIP, back is TMM↔pool member (null when unpaired).
    // Each side is { "client": <flow>, "server": <flow>|null }; a flow
    // carries src/dst ip+port, proto, packets, bytes, the TCP flag counters
    // (tcp_syn, tcp_rst, tcp_rst_after_bytes, …), the TLS observations
    // (tls_sni, tls_version, tls_alpn, tls_alert_desc, tls_cert_subject, …),
    // the HTTP observations (http_method, http_host, http_uri, http_path,
    // http_query, http_response_code, http_response_headers, …),
    // "f5_reset_causes", and the F5-trailer peer tuple (peer_remote_ip, …).
    "front": { "client": { "src_ip": "1.2.3.4", "src_port": 11111,
                           "dst_ip": "5.6.7.8", "dst_port": 443,
                           "proto": "tcp", "tls_sni": "api.example.com",
                           "http_method": "GET", "http_uri": "/v1/health" },
               "server": null },
    "back": null
  },
  "matched_vs": "/partition/vs_app",
  "partition": "partition",
  "profile_chain": ["tcp (lab_tcp)", "client_ssl (lab_clientssl_valid)",
                    "http (lab_http)"],
  "pool_selected": "10.0.0.10:8080",   // observed on the back side
  "snat_observed": "10.0.0.5:22222",   // observed, "" when not SNATted
  "event_sequence": ["rule_sni::CLIENTSSL_CLIENTHELLO",
                     "rule_route::HTTP_REQUEST"],   // "<rule>::<EVENT>"
  "event_blocks": [
    { "rule": "rule_sni", "event": "CLIENTSSL_CLIENTHELLO",
      "body": "if { [SSL::extensions exists -type 0] } { ... }\n... (truncated)" }
  ],
  "event_annotations": [
    { "rule": "rule_route", "event": "HTTP_REQUEST",
      "annotations": [ { "line": "if { [HTTP::host] equals \"api.example.com\" }",
                         "command": "HTTP::host",
                         "value": "api.example.com" } ] }
  ],
  "ltm_policies": ["/partition/lab_policy_rewrite"],
  "policy_decisions": [
    { "policy": "/partition/lab_policy_rewrite", "strategy": "first-match",
      "rules": [
        { "rule": "api_route", "ordinal": 1, "matched": true, "fired": true,
          "conditions": [
            { "operand": "http-host", "selector": "host", "operator": "equals",
              "expected": ["api.example.com"], "actual": "api.example.com",
              "matched": true, "name": "", "negate": false,
              "event": "request", "note": "" }
          ],
          "actions": [
            { "target": "forward", "verb": "select",
              "value": "/partition/lab_pool_api" }
          ] }
      ] }
  ],
  "apm_profile": "",
  "gtm_wide_ips": ["/Common/app.example.com"],  // the config's wide IPs
  "explain_text": "…",           // `f5 explain virtual` text for matched_vs
  "reset_analysis": "graceful FIN teardown (no RST)",
  "simulated_pool": "/partition/lab_pool_api",   // "" unless simulate=true
  "simulated_node": "10.0.0.10:8080",
  "simulated_response_committed": false,
  "simulated_logs": [],
  "simulated_decisions": [ { "category": "lb", "action": "pool_select",
                             "value": "/partition/lab_pool_api" } ],
  "simulation_error": ""
}
```

## Narrating a session

1. Open with the 5-tuple and `matched_vs` — the one-line gist.
2. `session.front` / `session.back`: what the client sent and what came back,
   or how far the connection got (HTTP and TLS fields, byte and packet
   counts).
3. `profile_chain`: which BIG-IP code paths ran (TCP-only, TLS-decrypt,
   HTTP-aware); order is attach order.
4. `event_sequence` + `event_annotations`: which branches were taken and what
   they read — each annotation is "in event X the iRule looked at command Y
   and saw Z", which explains a `[HTTP::host] equals` branch or an SNI route.
5. `policy_decisions`: which LTM policy rule fired and what it did. Every
   rule is listed with `matched` / `fired` and its full per-condition trace
   (`expected` beside `actual`, plus `note` for a condition that could not be
   evaluated) — narrate the fired ones and reach for the rest only to explain
   why nothing matched. A rule with zero conditions always matches, so a
   "default" rule fires under first-match / all-match whenever no earlier
   rule won. `best-match` is reported as `best-match-approx` ("most
   conditions wins"; F5's operand-specificity weighting is not reproduced).
6. `event_blocks`: consult only when you need to quote Tcl; already
   truncated — do not ask for `max_event_body_lines > 20` without cause.
7. `pool_selected` + `snat_observed` are observed. If `simulated_pool`
   differs, say so: the capture usually pre-dates the rule edit or another
   rule overrode the choice.
8. `reset_analysis` explains why the session ended; an RST with an F5 reset
   cause (e.g. `POOL_DOWN`, in the flow's `f5_reset_causes`) is definitive.
9. The `simulated_*` fields, when `simulate=true`, are the truth source (the
   iRule run under c-tcl with the captured state); on a non-empty
   `simulation_error` fall back to the static analysis.

## Limitations

- `tshark_filter` routes extraction through tshark; the F5 trailer
  peer-tuples that pair `:np` front/back sides come only from the built-in
  walker, so a filtered run loses front/back pairing. Drop the filter for
  proxied traffic.
- `simulate` needs `tclsh` on PATH and one orchestrator subprocess per
  matched session; avoid on captures with hundreds of sessions unless
  `tshark_filter` is set.
- Static event ordering comes from attached profiles and observed L7
  features; it does not honour `event disable` or conditional
  `when ... { return }` — consult the `simulated_*` fields for runtime truth.
- Path-through-iRule analysis is static unless `simulate=true`: it surfaces
  the relevant `when` bodies, it does not follow branches on payload bytes.
- GTM probe results and APM session state are not retained; `gtm_wide_ips`
  is the config's wide-IP inventory repeated on each session, not a
  per-session resolution.
- LTM policy evaluation covers operands `http-host`, `http-uri`
  (host/path/query), `http-method`, `http-header` (named), `ssl-extension
  server-name`, `tcp address`, and actions `forward select`, `http-reply
  redirect`, `http-uri replace`, `http-header insert/remove`, `tcp reset`.
  Other operands (cookie, geoip, ssl-cert, rate-limit) parse but evaluate
  as no-match with a `note`.

## Keeping the answer small

The report is complete, not pruned: a large capture returns a session per
5-tuple with every flow field on each. Narrow with `tshark_filter` and lower
`max_event_body_lines` rather than quoting the JSON back at the user; quote
only the fields the question turns on.

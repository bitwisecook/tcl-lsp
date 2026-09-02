---
name: irule-xc
description: "Translate an F5 BIG-IP iRule to F5 Distributed Cloud (XC) configuration. Produces Terraform HCL and JSON API output with coverage analysis. Highlights untranslatable constructs and suggests XC alternatives. Use when migrating iRules to F5 XC, converting BIG-IP iRules to Distributed Cloud, translating iRule logic to Terraform, or planning F5 XC migration."
allowed-tools: mcp__tcl-lsp__xc_translate, Bash, Read, Write
---

# iRule to F5 XC Translation

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. Call `mcp__tcl-lsp__xc_translate` with the contents as `source`
   (`output_format` default `both`): Terraform HCL, ves.io JSON, a coverage
   percentage, and a per-command breakdown of translated / untranslatable /
   advisory constructs. On a tool error report it and suggest fixes.
3. Coverage ≥ 80 %: the static translation stands. Below that, suggest XC
   alternatives for the untranslatable constructs: complex routing → custom
   route objects; L4 logic → App Stack; `table` / `session` state → XC rate
   limiting or API discovery; SSL/TLS events → load-balancer TLS config;
   WAF/ASM events → App Firewall or WAF exclusion rules; bot defence → Bot
   Defence; rate limiting → Rate Limiting.
4. Write `$FILE.tf` and `$FILE.xc.json`, check the Terraform parses, and
   comment each mapping from the original iRule.

## XC mapping

| iRule | XC |
|---|---|
| `pool <name>` | `volterra_origin_pool` + route |
| `switch [HTTP::path]` / `[HTTP::host]` | L7 routes with path (prefix, suffix, exact, regex) / domain matching |
| `if [HTTP::path] starts_with/ends_with/contains/matches_regex` | route path matching |
| `HTTP::header value "X"` eq / `HTTP::header exists` | route or policy header match / presence |
| `HTTP::cookie "X"` eq / `HTTP::cookie exists` | route or policy cookie match / presence |
| `[HTTP::query] contains` | query parameter matching |
| `[IP::client_addr] eq` / `class match [IP::client_addr] equals DG` | service policy source IP / IP prefix set |
| `! [cond]`, `ne` | `invert_matcher = true` |
| `cond1 \|\| cond2` | one rule per OR branch |
| `HTTP::redirect` | `redirect_route` |
| `HTTP::respond 403/401` / `200` | `volterra_service_policy` deny rule / `direct_response_route` |
| `HTTP::header insert/replace/remove` | load balancer header processing |
| `ASM::disable` / `ASM::enable` | WAF exclusion rule with `app_firewall_detection_control` / no action |
| `class match` | service policy rules (data-group entries) |
| `RULE_INIT`, `CLIENT_ACCEPTED`, `TCP::*`, `UDP::*`, `eval`, `uplevel` | no equivalent (static config / L4 / App Stack) |
| `CLIENTSSL_HANDSHAKE`, `ASM_*` events | XC TLS settings, XC App Firewall |

`app_firewall_detection_control` actions: `exclude_all_attack_types`
(default), `exclude_attack_type_contexts`, `exclude_signature_contexts`,
`exclude_bot_name_contexts`, `exclude_violation_contexts`.

$ARGUMENTS

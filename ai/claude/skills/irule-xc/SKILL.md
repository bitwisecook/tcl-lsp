---
name: irule-xc
description: "Translate an F5 BIG-IP iRule to F5 Distributed Cloud (XC) configuration. Produces Terraform HCL and JSON API output with coverage analysis. Highlights untranslatable constructs and suggests XC alternatives. Use when migrating iRules to F5 XC, converting BIG-IP iRules to Distributed Cloud, translating iRule logic to Terraform, or planning F5 XC migration."
allowed-tools: mcp__tcl-lsp__xc_translate, Bash, Read, Write
---

# iRule to F5 XC Translation

Translate an iRule to F5 XC routes, service policies, origin pools, WAF exclusion rules, and header processing.

## Steps

1. Read domain knowledge from `../_prompts/irules_system.md`
2. Read the iRule source file
3. Run the static translator: call the `mcp__tcl-lsp__xc_translate` MCP
   tool, passing the iRule's contents as the `source` argument (leave
   `output_format` at its default `both` to get Terraform HCL + ves.io
   JSON). The tool returns the translated Terraform, the JSON API
   config, an overall coverage percentage, and a per-command breakdown
   of translated / untranslatable / advisory constructs (each with its
   iRule command and XC description).
4. If the tool fails (e.g. parse error), report the error clearly and suggest fixes
5. Review the output:
   - If coverage >= 80%, the static translation is sufficient
   - If coverage < 80%, review untranslatable constructs and suggest alternatives:
     - For complex routing: suggest XC custom route objects
     - For L4 logic: suggest App Stack containers
     - For state management (table, session): suggest XC-native rate limiting or API discovery
     - For SSL/TLS events: suggest XC TLS configuration on the load balancer
     - For WAF/ASM events: suggest XC App Firewall or WAF exclusion rules
     - For bot defence: suggest XC Bot Defence
     - For rate limiting patterns: suggest XC Rate Limiting
6. Write Terraform output to `$FILE.tf`
7. Write JSON API output to `$FILE.xc.json`
8. Validate the generated Terraform is syntactically valid
9. Add comments explaining each mapping from the original iRule to XC constructs

## XC Mapping Reference

| iRule Construct | XC Equivalent |
|---|---|
| `pool <name>` | `volterra_origin_pool` + route |
| `switch [HTTP::path]` | L7 routes with path matching (prefix, suffix, exact, regex) |
| `switch [HTTP::host]` | L7 routes with domain matching |
| `if [HTTP::path] starts_with/ends_with/contains/matches_regex` | Route path matching |
| `if [HTTP::header value "X"] eq "Y"` | Route or policy header matching |
| `if [HTTP::header exists "X"]` | Route or policy header presence matching |
| `if [HTTP::cookie "X"] eq "Y"` | Route or policy cookie matching |
| `if [HTTP::cookie exists "X"]` | Route or policy cookie presence matching |
| `if [HTTP::query] contains "X"` | Route or policy query parameter matching |
| `if [IP::client_addr] eq "X"` | Service policy client source IP matching |
| `if [class match [IP::client_addr] equals DG]` | Service policy IP prefix set matching |
| `! [condition]` or `[condition] ne "X"` | Inverted match (`invert_matcher = true`) |
| `cond1 \|\| cond2` | Multiple rules (one per OR branch) |
| `HTTP::redirect` | `redirect_route` |
| `HTTP::respond 403/401` | `volterra_service_policy` deny rule |
| `HTTP::respond 200` | `direct_response_route` |
| `HTTP::header insert/replace/remove` | Load balancer header processing |
| `ASM::disable` | WAF exclusion rule with `app_firewall_detection_control` |
| `ASM::enable` | No action (WAF enabled by default) |
| `class match` | Service policy rules (data-group entries) |
| `RULE_INIT` | No equivalent -- use static XC config |
| `CLIENT_ACCEPTED` | No equivalent -- L4 event |
| `CLIENTSSL_HANDSHAKE` | XC TLS settings |
| `ASM_*` events | XC App Firewall |
| `eval`, `uplevel` | No equivalent -- consider App Stack |
| `TCP::*`, `UDP::*` | No equivalent -- L4 commands |

## WAF Exclusion Rule Actions

When `ASM::disable` is translated to a WAF exclusion rule, these actions can be configured
in the `app_firewall_detection_control` block:

| Action | Description |
|---|---|
| `exclude_all_attack_types` | Disable all WAF detection (current default) |
| `exclude_attack_type_contexts` | Exclude specific attack types with context |
| `exclude_signature_contexts` | Exclude specific WAF signature IDs |
| `exclude_bot_name_contexts` | Exclude specific bot names from detection |
| `exclude_violation_contexts` | Exclude specific violation types |

$ARGUMENTS

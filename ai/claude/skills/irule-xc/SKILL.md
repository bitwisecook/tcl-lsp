---
name: irule-xc
description: >
  Translate an F5 BIG-IP iRule to F5 Distributed Cloud (XC) configuration.
  Produces Terraform HCL and JSON API output with coverage analysis.
  Highlights untranslatable constructs and suggests XC alternatives.
allowed-tools: Bash, Read, Write
---

# iRule to F5 XC Translation

Translate an iRule to F5 XC routes, service policies, origin pools, WAF exclusion rules, and header processing.

## Steps

1. Read domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule source file
3. Run the static translator:
   ```bash
   uv run --no-dev python -c "
   import json
   from core.commands.registry.runtime import configure_signatures
   configure_signatures(dialect='f5-irules')
   from core.xc.translator import translate_irule
   from core.xc.terraform import render_terraform
   from core.xc.json_api import render_json

   source = open('$FILE').read()
   result = translate_irule(source)
   print('=== TERRAFORM ===')
   print(render_terraform(result))
   print('=== JSON API ===')
   print(json.dumps(render_json(result), indent=2))
   print('=== COVERAGE ===')
   print(f'{result.coverage_pct:.1f}%')
   if result.untranslatable_count > 0:
       print('=== UNTRANSLATABLE ===')
       for item in result.items:
           if item.status.name == 'UNTRANSLATABLE':
               note = f' ({item.note})' if item.note else ''
               print(f'- {item.irule_command}: {item.xc_description}{note}')
   if result.advisory_count > 0:
       print('=== ADVISORY ===')
       for item in result.items:
           if item.status.name == 'ADVISORY':
               print(f'- {item.irule_command}: {item.xc_description}')
   "
   ```
4. Review the output:
   - If coverage >= 80%, the static translation is sufficient
   - If coverage < 80%, review untranslatable constructs and suggest alternatives:
     - For complex routing: suggest XC custom route objects
     - For L4 logic: suggest App Stack containers
     - For state management (table, session): suggest XC-native rate limiting or API discovery
     - For SSL/TLS events: suggest XC TLS configuration on the load balancer
     - For WAF/ASM events: suggest XC App Firewall or WAF exclusion rules
     - For bot defence: suggest XC Bot Defence
     - For rate limiting patterns: suggest XC Rate Limiting
5. Write Terraform output to `$FILE.tf`
6. Write JSON API output to `$FILE.xc.json`
7. Validate the generated Terraform is syntactically valid
8. Add comments explaining each mapping from the original iRule to XC constructs

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
| `RULE_INIT` | No equivalent — use static XC config |
| `CLIENT_ACCEPTED` | No equivalent — L4 event |
| `CLIENTSSL_HANDSHAKE` | XC TLS settings |
| `ASM_*` events | XC App Firewall |
| `eval`, `uplevel` | No equivalent — consider App Stack |
| `TCP::*`, `UDP::*` | No equivalent — L4 commands |

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

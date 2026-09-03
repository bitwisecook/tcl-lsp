---
name: irule-datagroup
description: "Analyse an F5 iRule for opportunities to extract inline lookup patterns into BIG-IP data-groups. Uses the static extraction engine for mechanical conversions and AI reasoning for complex patterns. Type-aware: detects IP/CIDR, integer, and string data-groups. Use when extracting data-groups from iRules, optimising iRule lookup performance, converting inline iRule patterns to data-groups, or refactoring iRule switch/if chains."
allowed-tools: mcp__tcl-lsp__suggest_datagroup_extractions, mcp__tcl-lsp__extract_datagroup, Read, Edit
---

# iRule Data-Group Analysis

## Steps

1. Read `../_prompts/irules_system.md` (data-group reference), then the
   iRule.
2. Call `mcp__tcl-lsp__suggest_datagroup_extractions` with the contents as
   `source`. Each candidate carries its pattern (if_chain, switch, or_chain),
   inferred value type (ip, integer, string), CIDR flag, body shape
   (identical, set_mapping, return_mapping, complex), and confidence. On a
   tool error report it and suggest fixes.
3. **High confidence:** call `mcp__tcl-lsp__extract_datagroup` with `source`
   plus the candidate's `line` and `character` (optionally `dg_name`); it
   returns the rewritten iRule and the tmsh definition.
4. **Medium / low:** decide by hand — a domain-appropriate name, whether to
   consolidate related patterns, the `class match` operator (equals,
   contains, starts_with), CIDR handling for ip types — and write the
   definition.
5. For each extraction show the inline code, the `class match` /
   `class lookup` replacement, the tmsh definition
   (`ltm data-group internal <name> { records { <key> { data <value> } } type <string|ip|integer> }`),
   and the performance benefit; then apply with Edit. If nothing qualifies,
   say why the current approach is acceptable.

## Reference

| Type | Examples | Operators |
|---|---|---|
| string | `"/api/v1"`, `"example.com"` | `equals`, `starts_with`, `contains` |
| ip | `10.0.0.0/8`, `192.168.1.1`, `::1` | `equals` (CIDR-aware) |
| integer | `80`, `443` | `equals` |

Membership test (identical bodies) → `if { [class match $host equals allowed_hosts] } { pool web_pool }`.
Value lookup (different bodies) → `pool [class lookup $uri uri_pool_map]`.
IP allowlist → `if { [class match [IP::client_addr] equals trusted_networks] } { ... }`.

$ARGUMENTS

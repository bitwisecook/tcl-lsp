---
name: irule-datagroup
description: >
  Analyse an iRule for opportunities to extract inline lookup patterns
  into BIG-IP data-groups. Uses the static extraction engine for
  mechanical conversions and AI reasoning for complex patterns.
  Type-aware: detects IP/CIDR, integer, and string data-groups.
allowed-tools: Bash, Read, Edit
---

# iRule Data-Group Analysis

Analyse an iRule for data-group extraction opportunities.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md` (includes data-groups reference)
2. Read the iRule file to analyse
3. Run the AI-enhanced data-group scanner to find all candidates:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py suggest-datagroups $FILE
   ```
   This returns structured context for each candidate:
   - Pattern type (if_chain, switch, or_chain)
   - Inferred value type (ip, integer, string)
   - Whether CIDR notation is present
   - Body shape (identical, set_mapping, return_mapping, complex)
   - Confidence level (high = mechanical, medium = needs review, low = use judgement)
4. For **high-confidence** candidates, run the static extractor:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py extract-datagroup $FILE --line <LINE>
   ```
   This produces both the rewritten iRule code and the tmsh data-group definition.
5. For **medium/low-confidence** candidates, use AI reasoning to:
   - Choose an appropriate data-group name based on the domain context
   - Decide whether to consolidate related patterns into a single data-group
   - Determine the correct `class match` operator (equals, contains, starts_with)
   - Handle CIDR ranges correctly (IP data-groups with network prefixes)
   - Generate the data-group definition with proper typing
6. For each extraction, provide:
   - The original inline code
   - The replacement using `class match` / `class lookup`
   - The complete tmsh data-group definition:
     ```
     ltm data-group internal <name> {
         records {
             <key> { data <value> }
         }
         type <string|ip|integer>
     }
     ```
   - The performance benefit explanation
7. Apply the changes to the file using the Edit tool
8. If no data-group opportunities exist, explain why the current approach is acceptable

## Data-group type reference

| Value type | Examples | class operator |
|------------|----------|----------------|
| string | `"/api/v1"`, `"example.com"` | `equals`, `starts_with`, `contains` |
| ip | `10.0.0.0/8`, `192.168.1.1`, `::1` | `equals` (supports CIDR matching) |
| integer | `80`, `443`, `8080` | `equals` |

## Common patterns

### Membership test (identical bodies)
```tcl
# Before: if/elseif chain
if { $host eq "a.com" } { pool web_pool } elseif { $host eq "b.com" } { pool web_pool }

# After: class match
if { [class match $host equals allowed_hosts] } { pool web_pool }
```

### Value lookup (different bodies)
```tcl
# Before: switch mapping
switch -exact -- $uri {
    "/api" { pool api_pool }
    "/web" { pool web_pool }
}

# After: class lookup
pool [class lookup $uri uri_pool_map]
```

### IP allowlist with CIDR
```tcl
# Before: nested if
if { [IP::addr [IP::client_addr] equals 10.0.0.0/8] } { ... }

# After: IP data-group (handles CIDR natively)
if { [class match [IP::client_addr] equals trusted_networks] } { ... }
```

$ARGUMENTS

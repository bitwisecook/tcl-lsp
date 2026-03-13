# KCS: feature — Refactor: Extract to Data-Group

## Summary

Extract hardcoded if/elseif chains or switch statements with literal values into F5 BIG-IP data-group lookups, with type-aware inference for IP/CIDR (IPv4 + IPv6), integer, and string values.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

### Editor (all editors via LSP)

Place the cursor on an `if` chain or `switch -exact` with literal value comparisons. Trigger code actions and choose **"Extract to data-group"**. The code action includes the tmsh data-group definition as a comment block.

Only available when the document dialect is iRules.

### MCP

| Tool | Description |
|------|-------------|
| `extract_datagroup` | Static extraction at a specific line — returns rewritten source + tmsh definition |
| `suggest_datagroup_extractions` | AI-enhanced scan of entire source — returns candidates with confidence scores |

### Claude Code

- `suggest-datagroups <file>` — scan for all data-group extraction candidates
- `extract-datagroup <file> --line N` — extract at a specific line
- `/irule-datagroup <file>` — AI-enhanced analysis with LLM reasoning

## Before / After

### Membership (string)

#### Before

```tcl
when HTTP_REQUEST {
    set host [HTTP::host]
    if {$host eq "app.example.com"} {
        pool app_pool
    } elseif {$host eq "api.example.com"} {
        pool app_pool
    } elseif {$host eq "web.example.com"} {
        pool app_pool
    } elseif {$host eq "cdn.example.com"} {
        pool app_pool
    }
}
```

#### After

```tcl
when HTTP_REQUEST {
    set host [HTTP::host]
    if {[class match $host equals allowed_hosts]} {
        pool app_pool
    }
}
```

```text
ltm data-group internal allowed_hosts {
    type string
    records {
        "app.example.com" { }
        "api.example.com" { }
        "web.example.com" { }
        "cdn.example.com" { }
    }
}
```

### IP/CIDR blocklist (IPv4 + IPv6)

#### Before

```tcl
when CLIENT_ACCEPTED {
    set addr [IP::client_addr]
    if {$addr eq "10.0.0.0/8"} {
        drop
    } elseif {$addr eq "172.16.0.0/12"} {
        drop
    } elseif {$addr eq "192.168.0.0/16"} {
        drop
    } elseif {$addr eq "2001:db8::/32"} {
        drop
    } elseif {$addr eq "fd00::/8"} {
        drop
    }
}
```

#### After

```tcl
when CLIENT_ACCEPTED {
    set addr [IP::client_addr]
    if {[class match $addr equals blocked_networks]} {
        drop
    }
}
```

```text
ltm data-group internal blocked_networks {
    type ip
    records {
        10.0.0.0/8 { }
        172.16.0.0/12 { }
        192.168.0.0/16 { }
        2001:db8::/32 { }
        fd00::/8 { }
    }
}
```

IPv4, IPv6, CIDR notation, and IPv4-mapped IPv6 addresses (`::ffff:10.0.0.1`) are all detected and produce `type ip` data-groups.

### Value mapping (switch)

#### Before

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    switch -exact -- $uri {
        "/api"    { set target api_pool }
        "/web"    { set target web_pool }
        "/static" { set target cdn_pool }
        "/admin"  { set target mgmt_pool }
    }
    pool $target
}
```

#### After

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    set target [class lookup $uri uri_pool_map]
    pool $target
}
```

```text
ltm data-group internal uri_pool_map {
    type string
    records {
        "/api" {
            data "api_pool"
        }
        "/web" {
            data "web_pool"
        }
        "/static" {
            data "cdn_pool"
        }
        "/admin" {
            data "mgmt_pool"
        }
    }
}
```

When every arm sets the same variable to different values, the refactoring produces a `class lookup` with a value-mapping data-group.

## Data-group type inference

| Values | Inferred type | Example |
|--------|---------------|---------|
| IPv4 addresses or CIDR | `ip` | `10.0.0.0/8`, `192.168.1.1` |
| IPv6 addresses or CIDR | `ip` | `2001:db8::/32`, `fe80::1`, `fd00::/8` |
| IPv4-mapped IPv6 | `ip` | `::ffff:10.0.0.1` |
| Integers | `integer` | `80`, `443`, `8080` |
| Everything else | `string` | `"example.com"`, `"/api"` |

## Body shape detection

| Shape | Pattern | Result |
|-------|---------|--------|
| `identical` | All arms have the same body | `class match` (membership test) |
| `set_mapping` | All arms `set var value` (same var, different values) | `class lookup` (value mapping) |
| `return_mapping` | All arms `return value` | `class lookup` (value mapping) |
| `complex` | Mixed or multi-statement bodies | Not extracted (returns `None`) |

## Operational context

The refactoring supports two source patterns:

1. **if/elseif chains** — branches testing `$var eq "literal"`, including OR-chains (`$var eq "a" || $var eq "b"`)
2. **switch -exact** — each arm is a literal key

Type inference uses Python's `ipaddress` module for IP/CIDR detection, which natively supports both IPv4 and IPv6 address families. The AI-enhanced suggestion tool returns structured context including pattern type, variable name, inferred type, CIDR presence, body shape, confidence level, and a pre-computed static result for each candidate.

## File-path anchors

- `core/refactoring/_extract_datagroup.py`
- `lsp/features/code_actions.py`
- `ai/mcp/tcl_mcp_server.py`
- `ai/claude/skills/irule-datagroup/SKILL.md`

## Failure modes

- Fewer than 2 distinct literal values (not worth extracting).
- Complex body shape — multi-statement or mixed patterns across arms.
- Branches test different variables.
- Non-literal test values (variables, command substitutions).

## Test anchors

- `tests/test_refactoring.py::TestExtractDatagroup`
- `tests/test_refactoring.py::TestSuggestDatagroupExtraction`

## Samples

- `30-extract-datagroup-before.irul` — string membership before
- `30-extract-datagroup-after.irul` — string membership after
- `31-extract-datagroup-ip-before.irul` — IPv4+IPv6 blocklist before
- `31-extract-datagroup-ip-after.irul` — IPv4+IPv6 blocklist after
- `32-extract-datagroup-mapping-before.irul` — value mapping before
- `32-extract-datagroup-mapping-after.irul` — value mapping after

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)
- [iRule data-group skill](../../../ai/claude/skills/irule-datagroup/SKILL.md)

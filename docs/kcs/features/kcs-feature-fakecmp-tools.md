# KCS: feature — FakeCMP Tools

> **Audience:** User
> **Type:** Functionality

## Summary

Deterministic TMM hash lookup and multi-TMM test distribution planner for iRule testing without hardware.

## Applies to

MCP

## Question

How do I find out which TMM a connection lands on, and how do I plan test traffic for multi-TMM coverage?

## How to use

Two MCP tools cover multi-TMM test planning:

| Tool | What it does |
|------|-------------|
| `fakecmp_which_tmm` | Maps a connection four-tuple (source address, source port, destination address, destination port) to a TMM identifier using a deterministic hash. This is not the real BIG-IP CMP algorithm — it is a repeatable approximation for test design. |
| `fakecmp_suggest_sources` | Generates a mapping of client address and port combinations that land on each TMM, ensuring multi-TMM test coverage for a given TMM count. |

### MCP

```json
{"tool": "fakecmp_which_tmm", "arguments": {
  "tmm_count": 4,
  "src_addr": "10.0.0.1", "src_port": 12345,
  "dst_addr": "192.168.1.100", "dst_port": 443
}}
```

Returns the TMM identifier the connection hashes to.

## Example

Planning test traffic across 4 TMMs:

```json
{"tool": "fakecmp_suggest_sources", "arguments": {
  "tmm_count": 4,
  "dst_addr": "192.168.1.100", "dst_port": 443
}}
```

Returns one suggested source address and port per TMM so a test harness can exercise `static::` variable isolation across all TMM instances.

## Related

- [KCS feature index](README.md)
- [iRule Test Framework](../../design/contracts/irule-test-framework.md) — TMM simulation for testing iRules
- [MCP Server](kcs-feature-mcp-server.md)

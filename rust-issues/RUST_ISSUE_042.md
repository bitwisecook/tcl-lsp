# RUST_ISSUE_042: `Network::parse` has no route-domain `%N` handling, so every route-domain-qualified CIDR fails to parse and typed fields silently become `None`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-bigip/src/value/network.rs:99` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip/src/value/network.rs:99 — `Network::parse` has no route-domain `%N` handling, so every route-domain-qualified CIDR fails to parse and typed fields silently become `None`.
`net self /Common/s1 { address 10.1.1.1%1/24 }` → `obj.address = None`; likewise `net route { network 10.2.0.0%1/16 }` and `network default%1` (also loses `is_default_route`). `IPAddress::parse`/`Destination`/pcap_enrich.rs:504 all strip `%rd` — only the model parser path was missed. Confidence: high

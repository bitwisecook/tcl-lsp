# RUST_ISSUE_194: unbracketed-IPv6 destination parsing unconditionally treats the last `.` as a port separator, so a portless IPv4-mapped address fails to parse as a Destination

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/tcl-bigip-query/src/builtins/net.rs:1002-1033` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-bigip-query/src/builtins/net.rs:1002-1033 — unbracketed-IPv6 destination parsing unconditionally treats the last `.` as a port separator, so a portless IPv4-mapped address fails to parse as a Destination.
`with_port("::ffff:10.1.1.1"; 443)` errors "cannot parse destination" (rfind('.') splits off `1` as port and `::ffff:10.1.1` fails `TypedIp` parse, with no no-port retry), while the equivalent `2001:db8::1` works; `is_wildcard_port`/`with_host` are likewise affected. Quote: `match addr_part.rfind('.') { ... Some(sep) => { let addr_text = &addr_part[..sep]; ... }`.
Confidence: medium

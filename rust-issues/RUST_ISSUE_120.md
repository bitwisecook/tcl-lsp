# RUST_ISSUE_120: `host_count` / `ip_range_count` cast a u128 count to i64 by truncation, returning negative/garbage values for large IPv6 networks and ranges

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/tcl-bigip-query/src/builtins/net.rs:1608 (and 2278)` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip-query/src/builtins/net.rs:1608 (and 2278) — `host_count` / `ip_range_count` cast a u128 count to i64 by truncation, returning negative/garbage values for large IPv6 networks and ranges.
`host_count("2001:db8::/32")` computes 2^96−2 and returns `-2`; any range spanning ≥2^63 addresses is similarly wrong with no error. Quote: `let count = if total <= 2 { total } else { total - 2 }; Ok(Value::Int(count as i64))`.
Confidence: high

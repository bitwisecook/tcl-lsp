# RUST_ISSUE_166: `dict incr` uses `cur.wrapping_add(amount)`, silently wrapping at i64 bounds while scalar `incr` promotes to bignum. `dict set d k 9223372036854775807; dict incr d k` → `-9223372036854775808`; Tcl → `9223372036854775808`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_dict.rs:492` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

runtime/rust/src/cmd_dict.rs:492 — `dict incr` uses `cur.wrapping_add(amount)`, silently wrapping at i64 bounds while scalar `incr` promotes to bignum. `dict set d k 9223372036854775807; dict incr d k` → `-9223372036854775808`; Tcl → `9223372036854775808`. Confidence: high

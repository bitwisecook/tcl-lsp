# RUST_ISSUE_093: `dict incr` reads the stored/increment integer via a decimal-only `parse_i64`. `dict set d k 0x10; dict incr d k` → `expected integer but got "0x10"`; Tcl → `17`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_dict.rs:504` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

runtime/rust/src/cmd_dict.rs:504 — `dict incr` reads the stored/increment integer via a decimal-only `parse_i64`. `dict set d k 0x10; dict incr d k` → `expected integer but got "0x10"`; Tcl → `17`. Confidence: high

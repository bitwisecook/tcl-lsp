# RUST_ISSUE_192: `_fakecmp_hash` assumes dotted-quad addresses (`split $src_addr .`), so an IPv6 `client_addr` in auto mode throws a Tcl `expr` error instead of selecting a TMM

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-irule-test/tcl/orchestrator.tcl:1486` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-irule-test/tcl/orchestrator.tcl:1486 — `_fakecmp_hash` assumes dotted-quad addresses (`split $src_addr .`), so an IPv6 `client_addr` in auto mode throws a Tcl `expr` error instead of selecting a TMM. Confidence: high

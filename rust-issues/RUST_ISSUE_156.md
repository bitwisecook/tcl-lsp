# RUST_ISSUE_156: bounded requirement `min-max` misses Tcl's min==max special case

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/version.rs:89` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-registry/src/version.rs:89 — bounded requirement `min-max` misses Tcl's min==max special case.
Real Tcl `package vsatisfies 8.4 8.4-8.4` → 1; here `satisfies_one("8.4","8.4-8.4")` → false, so an exact-pin requirement never matches. Confidence: medium

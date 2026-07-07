# RUST_ISSUE_189: `NO_PARTITION_PREFIX` lists only `("auth","partition")`, so inherently unpartitioned kinds get a bogus `/Common/` prefix

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-bigip/src/parser/driver.rs:66` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-bigip/src/parser/driver.rs:66 — `NO_PARTITION_PREFIX` lists only `("auth","partition")`, so inherently unpartitioned kinds get a bogus `/Common/` prefix.
`net interface 1.1 { }` → `/Common/1.1`; `sys provision ltm { }` → `/Common/ltm` — lookups by the real name miss. Confidence: medium

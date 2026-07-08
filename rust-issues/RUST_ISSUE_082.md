# RUST_ISSUE_082: `DIALECT_NAMES` omits `TCL91` and `BPF`, so `tcl registry-dump` silently drops dialect membership

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/command_snapshot.rs:103` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-registry/src/command_snapshot.rs:103 — `DIALECT_NAMES` omits `TCL91` and `BPF`, so `tcl registry-dump` silently drops dialect membership.
A `TCL90_PLUS` spec serialises `"dialects": ["tcl9.0"]` (9.1 vanishes); a BPF-only spec serialises `"dialects": []` — indistinguishable from "available nowhere". Confidence: high

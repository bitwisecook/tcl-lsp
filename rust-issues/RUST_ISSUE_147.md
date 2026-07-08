# RUST_ISSUE_147: `guard_interval` computes `k - 1` / `k + 1` unchecked, which panics (attempt to add/subtract with overflow) in debug/test builds for a branch literal at the i64 boundary; release wraps to a sound-but-wider bound (no `overflow-checks` in the workspace release profile)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/intervals.rs:320-331` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/intervals.rs:320-331 — `guard_interval` computes `k - 1` / `k + 1` unchecked, which panics (attempt to add/subtract with overflow) in debug/test builds for a branch literal at the i64 boundary; release wraps to a sound-but-wider bound (no `overflow-checks` in the workspace release profile).
`if {$x > 9223372036854775807} { … lindex $l $x … }` — any analysis running the interval pass in a debug build panics; adversarial-input robustness issue only. Confidence: high

# RUST_ISSUE_171: integer `<<` (and `**`) wrap past the i128 bignum stand-in with a sign flip rather than promoting

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Bytecode VM |
| **Location** | `rust/tcl-vm/src/expr.rs:236` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-vm/src/expr.rs:236 — integer `<<` (and `**`) wrap past the i128 bignum stand-in with a sign flip rather than promoting.
`expr {1 << 127}` yields `-170141183460469231731687303715884105728` (tclsh: positive 2^127), because `1i128.wrapping_shl(127)` overflows to i128::MIN. [overlaps backend-parity expr overflow] Confidence: high

# RUST_ISSUE_171: integer `<<` (and `**`) wrap past the i128 bignum stand-in with a sign flip rather than promoting

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Bytecode VM |
| **Location** | `rust/tcl-vm/src/expr.rs:236` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-vm/src/expr.rs:236 — integer `<<` (and `**`) wrap past the i128 bignum stand-in with a sign flip rather than promoting.
`expr {1 << 127}` yields `-170141183460469231731687303715884105728` (tclsh: positive 2^127), because `1i128.wrapping_shl(127)` overflows to i128::MIN. [overlaps backend-parity expr overflow] Confidence: high

## Resolution

Closed alongside `RUST_ISSUE_011`: `<<`/`**` past the `i128` tier now promote to an arbitrary-precision bignum instead of wrapping with a sign flip (`big_arith`/`big_pow`). `incr`/`dict incr` share the same tower — `value_ops::int_add` keeps the `i128` fast path but falls back to `num-bigint` when an operand or the sum exceeds `i128` (so `incr` at `i128::MAX` yields `2**127`), a non-integer operand still erroring `expected integer but got …`. Verified against tclsh 9.0.4.

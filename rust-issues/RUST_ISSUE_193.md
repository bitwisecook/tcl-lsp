# RUST_ISSUE_193: the `add` builtin sums integers with unchecked `isum += i`, panicking in debug builds and silently wrapping in release, contradicting the crate's overflow hardening (the `+` operator uses `checked_add` and tests/hardening.rs:138 pins "clean error, never a debug panic")

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/tcl-bigip-query/src/builtins/mod.rs:974` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip-query/src/builtins/mod.rs:974 — the `add` builtin sums integers with unchecked `isum += i`, panicking in debug builds and silently wrapping in release, contradicting the crate's overflow hardening (the `+` operator uses `checked_add` and tests/hardening.rs:138 pins "clean error, never a debug panic").
`[9223372036854775807, 1] | add` → debug panic / release result `-9223372036854775808`. Quote: `Value::Int(i) => { isum += i; fsum += *i as f64; }`. Same unchecked pattern: `bi_range`'s `cur += step` (mod.rs:1170) and unary `-i` on `i64::MIN` (eval.rs:1338).
Confidence: high

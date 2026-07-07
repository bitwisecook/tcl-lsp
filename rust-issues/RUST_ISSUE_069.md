# RUST_ISSUE_069: left-shift folds an i64-overflowing result instead of declining; `checked_shl` only guards the shift *count*, not value overflow

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler front-end (segmenter/expr/subst) |
| **Location** | `rust/tcl-compiler/src/tcl_expr_eval.rs:478` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/tcl_expr_eval.rs:478 — left-shift folds an i64-overflowing result instead of declining; `checked_shl` only guards the shift *count*, not value overflow.
`expr {1 << 63}` yields bignum `9223372036854775808` in Tcl; contract is "value past a wide → None". Here `1i64.checked_shl(63)` returns `Some(i64::MIN)`, so O101 folds `1 << 63` to a wrong negative constant. (Add/Sub/Mul/Pow correctly return None on overflow; shift is the outlier.) Confidence: high

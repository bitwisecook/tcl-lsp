# RUST_ISSUE_075: the operand-drop identities that fold to an integer literal use the Double-inclusive numeric gate, so a proven-double operand yields an integer where Tcl yields a double

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Optimiser passes (inlining/taint/expr-simplify) |
| **Location** | `rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs:936,974` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs:936,974 — the operand-drop identities that fold to an integer literal use the Double-inclusive numeric gate, so a proven-double operand yields an integer where Tcl yields a double.
`reduce_arith_identity` `$x * 0 → 0` and `reduce_pow` `$x ** 0 → 1` (and `reduce_self_comparison` `$x - $x → 0`) fire when `$x` is typed Double. `set x [expr {$input*1.5}]; set y [expr {$x * 0}]` gives Tcl `0.0` but the rewrite produces `0`. The integer-only ops (reduce_shift/reduce_bitwise/reduce_mod, same predicate) also drop `$x << 0`/`$x & 0`/`$x % 1` for double `$x`, turning Tcl's "can't use floating-point value as operand" error into a value. Gates should require integer, not numeric. Confidence: high (mechanism), medium (frequency)

# RUST_ISSUE_070: arithmetic/unary/mathfunc operands go through `to_number`→`parse_literal`, which coerces boolean words (`true`/`yes`/`on`) to `1`/`0`; Tcl rejects them in numeric context

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler front-end (segmenter/expr/subst) |
| **Location** | `rust/tcl-compiler/src/tcl_expr_eval.rs:321-323` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/tcl_expr_eval.rs:321-323 — arithmetic/unary/mathfunc operands go through `to_number`→`parse_literal`, which coerces boolean words (`true`/`yes`/`on`) to `1`/`0`; Tcl rejects them in numeric context.
`expr {true + 0}` errors in Tcl but folds to `Int(1)` here, so O101 replaces an error with a value (`expr {yes * 2}` → `2`, `abs(true)` → `1`). The authors already fixed this for *comparisons* (`compare_numeric`→`strict_number`) but arith/unary/call still use the boolean-coercing `parse_literal`. Confidence: high

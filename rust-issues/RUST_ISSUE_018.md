# RUST_ISSUE_018: right-shift boundary is `y > 64` instead of `y >= 64`; at `y == 64` the code executes `x >> 64`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Compiler front-end (segmenter/expr/subst) |
| **Location** | `rust/tcl-compiler/src/tcl_expr_eval.rs:485` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

rust/tcl-compiler/src/tcl_expr_eval.rs:485 — right-shift boundary is `y > 64` instead of `y >= 64`; at `y == 64` the code executes `x >> 64`.
`expr {5 >> 64}` should fold to `0`. Instead the `else` runs `x >> 64` — a shift-overflow: **panics in debug/test builds** (`attempt to shift right with overflow`) and in release masks to `x >> 0` = `x`, folding `5 >> 64` to `5`. `if y > 64 { ... } else { Some(TclValue::Int(x >> y)) }` Confidence: high

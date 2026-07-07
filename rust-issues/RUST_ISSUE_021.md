# RUST_ISSUE_021: the taint transfer for `AssignExpr` uses only `join_uses` (variable SSA uses), so a taint source nested in an `[expr {…}]` command substitution is never propagated into the assigned variable

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Optimiser passes (inlining/taint/expr-simplify) |
| **Location** | `rust/tcl-compiler/src/taint.rs:724` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/taint.rs:724 — the taint transfer for `AssignExpr` uses only `join_uses` (variable SSA uses), so a taint source nested in an `[expr {…}]` command substitution is never propagated into the assigned variable.
`set x [expr {[gets stdin] + 1}]` lowers to `AssignExpr` with `[gets stdin]` retained as `ExprNode::Command` (lowering_hooks.rs:254-269). `join_uses` sees no `$var`, so `x` is clean; a later `eval $x` gets no T100. Asymmetry: `eval [expr {[gets stdin]}]` IS caught (via word_taint), but storing the value first launders the taint — false-negative security diagnostic. `Statement::AssignExpr { .. } => join_uses(uses, taints, ssa),` Confidence: high

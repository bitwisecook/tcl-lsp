# RUST_ISSUE_003: `fold_return_under_lattice` Path 3 builds the eval env flow-insensitively from *every* Const lattice entry ("preferring a non-zero version"), and the exit-version overlay only overrides when the exit version is itself Const — so a stale pre-loop/other-arm constant leaks into the fold

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | critical |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/optimiser/propagation.rs:903-915` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/optimiser/propagation.rs:903-915 — `fold_return_under_lattice` Path 3 builds the eval env flow-insensitively from *every* Const lattice entry ("preferring a non-zero version"), and the exit-version overlay only overrides when the exit version is itself Const — so a stale pre-loop/other-arm constant leaks into the fold.
`proc f {} { set x 0; foreach v {1 2} { set x $v }; return [expr {$x + 1}] }` — at the return, `x`'s exit version is a ConstSet phi (not overlaid), but `(x,1)=Const(0)` binds `x=0`; the fold yields `1`, and the argument-sensitive O103 path (propagation.rs:1425-1437, gated only on `summary.pure`) emits an applicable rewrite replacing `[f]` with `1` while tclsh returns `3`. Path 2's own comment ("Reading the precise version is what makes us bail on sum_list") documents the invariant Path 3 violates. Confidence: high

# RUST_ISSUE_015: barrier statements' own defs are never evaluated or widened (the barrier arm `continue`s before the defs loop, and the widen loop only touches keys already in `values`), so a barrier-defined variable stays Unknown and vanishes from phi joins

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/sccp.rs:480-496` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/sccp.rs:480-496 — barrier statements' own defs are never evaluated or widened (the barrier arm `continue`s before the defs loop, and the widen loop only touches keys already in `values`), so a barrier-defined variable stays Unknown and vanishes from phi joins.
`if {$c} { set x 5 } else { dict for {x y} $d {} }; if {$x == 5} {A} else {B}` — the `dict for` barrier defines `x@2` (defs_of "dict::for"), `values[(x,2)]` is never inserted, the merge phi joins `Const(5)` with Unknown → `Const(5)` → the second branch folds always-true (same O101/O107 consequences as above; also makes `sccp_constants_for`'s "all versions agree" test pass vacuously). Confidence: high

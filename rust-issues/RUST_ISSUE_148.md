# RUST_ISSUE_148: `refine_interval` applies the false-edge guard when the false target dominates the use, but a loop exit reached both by the header's false edge *and* by a `break` (with the guard variable un-redefined, hence no exit phi and matching versions) does not satisfy the guard on the break path

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/intervals.rs:465-476` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/intervals.rs:465-476 — `refine_interval` applies the false-edge guard when the false target dominates the use, but a loop exit reached both by the header's false edge *and* by a `break` (with the guard variable un-redefined, hence no exit phi and matching versions) does not satisfy the guard on the break path.
`set n 0; while {$n < 5} { if {[cond]} { break } }; lindex $l $n` — the exit refines `n` to `[5,+inf)` though the break path leaves `n == 0`; can produce false W230/W233 on degenerate loop shapes (the CFG builder's `if_next` edge-splitting protects all if/else merges, so only break-style merges are exposed). Confidence: medium

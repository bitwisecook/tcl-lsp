# RUST_ISSUE_067: `kill_all` inside the dominator-tree walk collapses the scope *stack* to a single root, so occurrences recorded after a kill land in the root scope and survive `pop_scope` (`len > 1` guard), leaking into sibling branches

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/gvn.rs:1025-1027 with 193-196` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/gvn.rs:1025-1027 with 193-196 — `kill_all` inside the dominator-tree walk collapses the scope *stack* to a single root, so occurrences recorded after a kill land in the root scope and survive `pop_scope` (`len > 1` guard), leaking into sibling branches.
`if {$c} { puts hi; set a [llength $lst] } else { set b [llength $lst] }` — the `puts` kill flattens scopes, `[llength $lst]` from the then-arm is inserted into root, and the else-arm's occurrence reports a false O105 "'llength $lst' computed again with the same arguments" although it was never computed on that path. Confidence: high

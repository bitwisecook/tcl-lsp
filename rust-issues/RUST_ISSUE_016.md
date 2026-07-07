# RUST_ISSUE_016: O125 sinks an assignment into a branch body past statements (and condition command-substitutions) that may redefine variables the assignment's RHS *reads*; only redefinition of the assigned variable itself is checked (`statement_defines_var`)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/optimiser/code_sinking.rs:63-104,267-290` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/optimiser/code_sinking.rs:63-104,267-290 — O125 sinks an assignment into a branch body past statements (and condition command-substitutions) that may redefine variables the assignment's RHS *reads*; only redefinition of the assigned variable itself is checked (`statement_defines_var`).
`set x [expr {$a + 1}]; if {$c} { set a 0; puts $x }` — grouped applicable edits delete the `set` and prepend it before `puts $x`, so `x` is now computed after `set a 0`: original prints `$a+1`, rewritten prints `1`. `find_deepest_targets`'s `no_prior_redefine` checks only `var` (`x`), never the RHS read-set, and nothing checks condition cmd-substs like `[regexp … -> a]`. Confidence: high

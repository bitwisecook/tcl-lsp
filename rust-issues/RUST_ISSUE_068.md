# RUST_ISSUE_068: `resolve_list_length` ignores every unresolvable phi incoming on the assumption it is an `lset` result ("lset preserves length"), but a length-*growing* def (`lappend`, `set l [concat …]`) is equally unresolvable and equally ignored, so the pre-loop length is trusted

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/interval_bounds.rs:83-95` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/interval_bounds.rs:83-95 — `resolve_list_length` ignores every unresolvable phi incoming on the assumption it is an `lset` result ("lset preserves length"), but a length-*growing* def (`lappend`, `set l [concat …]`) is equally unresolvable and equally ignored, so the pre-loop length is trusted.
`set l {a b c}; foreach i {1} { lappend l x y; lset l 5 v }` — the loop-header phi resolves `l`'s length to 3 (entry incoming), index interval [5,5] > 3 classifies `past_append`, and a false W231 fires though `lset l 5` is the legal append slot after the two lappends. Confidence: medium

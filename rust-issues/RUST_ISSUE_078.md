# RUST_ISSUE_078: so the destroy branch is dead for `unset` and `classify_variable_assignment` keys on the wrong word

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Analyser & diagnostics |
| **Location** | `rust/tcl-compiler/src/side_effects.rs:733 (assigns_variable_at) precedes :765 (DESTROYS_VARIABLE), and unset carries both assigns_variable_at: Some(0) and DESTROYS_VARIABLE` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/side_effects.rs:733 (assigns_variable_at) precedes :765 (DESTROYS_VARIABLE), and `unset` carries both `assigns_variable_at: Some(0)` and `DESTROYS_VARIABLE` — so the destroy branch is dead for `unset` and `classify_variable_assignment` keys on the wrong word.
`unset -nocomplain x` → keyed as a *write* to a variable named `-nocomplain` (x's destroy untracked); `unset x` → modeled as a *read* of x; `unset a b` → keys only `a`. Corrupts the effect model consumed by gvn/elimination/interprocedural/execution_intent. Confidence: high (mechanism), medium (diagnostic reach is indirect)

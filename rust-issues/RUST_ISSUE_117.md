# RUST_ISSUE_117: fluent `assert that decision ... was_called_with X` inspects only the *first* decision with the matching action (`break` on first match), unlike classic `assert_decision`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-irule-test/tcl/orchestrator.tcl:809` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-irule-test/tcl/orchestrator.tcl:809 — fluent `assert that decision ... was_called_with X` inspects only the *first* decision with the matching action (`break` on first match), unlike classic `assert_decision`.
An iRule that calls `pool a` then `pool b`: `assert that decision lb pool_select was_called_with "b"` fails — order-dependent false failures for repeated actions. Confidence: high

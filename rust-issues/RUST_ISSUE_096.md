# RUST_ISSUE_096: `int()` coerces its operand through f64 with no integer short-circuit, losing precision above 2^53

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Bytecode VM |
| **Location** | `rust/tcl-vm/src/cmd_math.rs:230` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-vm/src/cmd_math.rs:230 — `int()` coerces its operand through f64 with no integer short-circuit, losing precision above 2^53.
`set x 9007199254740993; expr {int($x)}` returns `9007199254740992` (tclsh: `9007199254740993`). Siblings guard this — m_round (272), m_wide (247), m_abs (218) all short-circuit integers first; m_int does not. Confidence: high

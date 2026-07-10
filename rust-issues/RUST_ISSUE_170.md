# RUST_ISSUE_170: `return -level N -code C` with N≥2 collapses to the code taking effect immediately (level 0), dropping the multi-level countdown

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Bytecode VM |
| **Location** | `rust/tcl-vm/src/command.rs:1234` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-vm/src/command.rs:1234 — `return -level N -code C` with N≥2 collapses to the code taking effect immediately (level 0), dropping the multi-level countdown.
`proc inner {} { return -level 2 -code error boom }` inside `catch {inner}` in the calling proc is seen by that catch as an immediate error (code 1) rather than the TCL_RETURN (code 2) tclsh delivers while the level decrements. `let final_code = if level == 0 { ret_code } else if ret_code == Code::Ok { Code::Return } else { ret_code };` Confidence: high

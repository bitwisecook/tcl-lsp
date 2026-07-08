# RUST_ISSUE_084: `file delete` (and `mkdir`, :252) declare `Arity::at_least(1)` in all dialects, but TIP 323 (Tcl 8.6+) made the zero-argument forms legal

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/commands/tcl/file_.rs:94` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-registry/src/commands/tcl/file_.rs:94 — `file delete` (and `mkdir`, :252) declare `Arity::at_least(1)` in all dialects, but TIP 323 (Tcl 8.6+) made the zero-argument forms legal.
A plain `file delete`/`file mkdir` under tcl8.6/9.x draws a false wrong-#-args; the bound is correct only for 8.4/8.5 and isn't dialect-split. Confidence: medium

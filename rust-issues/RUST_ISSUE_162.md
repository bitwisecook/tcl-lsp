# RUST_ISSUE_162: the bare `$name` scan consumes exactly two colons per `::` and stops at a third, but C Tcl consumes the entire colon run

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/lexer.rs:631-634` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/lexer.rs:631-634 — the bare `$name` scan consumes exactly two colons per `::` and stops at a third, but C Tcl consumes the entire colon run.
`$a:::b` — real Tcl reads variable `a:::b`; this lexer emits VAR `$a::` plus ESC `:b`, targeting the wrong variable. Confidence: high

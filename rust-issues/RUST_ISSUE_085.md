# RUST_ISSUE_085: `$arr(idx)` scanning counts balanced nested parens and gives backslash no effect, but C Tcl 8.4-9.1 terminates the index at the first `)` not consumed by a backslash/command/var token, with no paren nesting

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/lexer.rs:676-684 (scan_array_index_body, mirrored expr_lexer.rs:335-346)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/lexer.rs:676-684 (`scan_array_index_body`, mirrored expr_lexer.rs:335-346) — `$arr(idx)` scanning counts balanced nested parens and gives backslash no effect, but C Tcl 8.4-9.1 terminates the index at the first `)` not consumed by a backslash/command/var token, with no paren nesting.
`puts $a((b); puts done` — real Tcl ends the var after the first `)`; this lexer never reaches depth 0, emits a bogus `missing )` warning and swallows the rest of the source. Conversely `$a(x\)y)` — real Tcl index is `x\)y`; this scanner ends at the escaped `)`. Confidence: high

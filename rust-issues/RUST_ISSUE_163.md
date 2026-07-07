# RUST_ISSUE_163: `parse_command`'s `${…}` sub-scan stops at the first `}` with no backslash-pair/nesting handling, contradicting `parse_var`'s braced scan

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/lexer.rs:1117-1125` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/lexer.rs:1117-1125 — `parse_command`'s `${…}` sub-scan stops at the first `}` with no backslash-pair/nesting handling, contradicting `parse_var`'s braced scan.
`[set ${a\}] x}]` — real Tcl's braced name is `a\}] x`; this scanner ends the `${` at the escaped `}` and closes the CMD token at the `]` *inside* the name. Confidence: high

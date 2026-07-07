# RUST_ISSUE_165: `braced()` counts `{`/`}` with no backslash handling, but Tcl's brace scanning consumes `\X` pairs so `\}` does not close

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/expr_lexer.rs:420-443` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/expr_lexer.rs:420-443 — `braced()` counts `{`/`}` with no backslash handling, but Tcl's brace scanning consumes `\X` pairs so `\}` does not close.
Expr body `{a\}b} eq $x` — the String token ends at the escaped `}` (`{a\}`), leaving `b}` to lex as ident + stray `}`, degrading the expression to Raw. Confidence: high

# RUST_ISSUE_164: `recurse_wrapped` strips a trailing `}`/`]` whenever token text ends with one, but under the inner-end convention a non-empty token's text never includes its own closer

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/highlight.rs:243-246 (and collect_recurse 150-152)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/highlight.rs:243-246 (and collect_recurse 150-152) — `recurse_wrapped` strips a trailing `}`/`]` whenever token text ends with one, but under the inner-end convention a non-empty token's text never includes its own closer.
`{a{b}}` (Str span covers `{a{b}`) recurses on `a{b` instead of `a{b}`, mis-lexing the nested brace as unterminated and mis-colouring; same for `[a [b]]`. Strip should apply only when the stripped inner is empty. Confidence: high

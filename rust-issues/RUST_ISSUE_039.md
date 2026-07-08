# RUST_ISSUE_039: Minifier unbraces `${var}` unsafely: the "next token extends the name" guard is only applied to quoted args, and the guard itself ignores `(`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/minify.rs:2458-2466 and 2382-2391` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/minify.rs:2458-2466 and 2382-2391 — Minifier unbraces `${var}` unsafely: the "next token extends the name" guard is only applied to quoted args, and the guard itself ignores `(`.
`puts ${a}jumps` (bare word) minifies to `puts $ajumps` — now reads variable `ajumps`. And `puts "${x}(k)"` becomes `puts "$x(k)"` — an array-element substitution instead of scalar-plus-literal (`(` extends a `$` reference but the check is `c.is_alphanumeric() || c == '_'`; same `is_word_byte` blindspot at minify.rs:1587). Confidence: high

# RUST_ISSUE_105: Inlining a brace-quoted value into a double-quoted reference context turns literal text into live substitutions

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/refactor/inline_variable.rs:102-111` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/refactor/inline_variable.rs:102-111 — Inlining a brace-quoted value into a double-quoted reference context turns literal text into live substitutions.
`set x {a $b}` + `puts "v: $x"` (prints `v: a $b`) inlines to `puts "v: a $b"` — `$b` is now substituted. The interpolated path only rejects whitespace outside quotes; it never checks the dequoted content for `$`/`[`/`\`. Confidence: high

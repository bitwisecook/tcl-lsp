# RUST_ISSUE_103: A bare `$` inside a double-quoted string is re-emitted as `{$}`, changing the string's value

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/formatting/engine.rs:114-116 and minify.rs:2380` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/formatting/engine.rs:114-116 and minify.rs:2380 — A bare `$` inside a double-quoted string is re-emitted as `{$}`, changing the string's value.
The lexer emits a lone `$` as a `TokenType::Str` token, and both reconstructors do `TokenType::Str => format!("{{{}}}", …)` with no quoted-context awareness — `puts "cost: $"` formats/minifies to `puts "cost: {$}"` (prints `cost: {$}`). Confidence: high

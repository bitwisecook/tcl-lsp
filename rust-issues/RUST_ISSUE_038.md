# RUST_ISSUE_038: `format_switch_body` silently deletes comment lines inside a `switch` body

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/formatting/engine.rs:447` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/formatting/engine.rs:447 — `format_switch_body` silently deletes comment lines inside a `switch` body.
In the element loop, `TokenType::Comment => continue` drops the token and the output is rebuilt from elements only, so formatting `switch $x {\n  # note\n  a { puts 1 }\n}` erases `# note` entirely — irrecoverable user-text loss. The document-level path (`parse_commands`) preserves comments; only switch bodies lose them. Confidence: high

# RUST_ISSUE_104: Backslash-newline collapse inside double-quoted strings deletes pre-backslash spaces that are string data

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/formatting/engine.rs:80-89 (via append_word_arg 884-889)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/formatting/engine.rs:80-89 (via append_word_arg 884-889) — Backslash-newline collapse inside double-quoted strings deletes pre-backslash spaces that are string data.
`normalise_backslash_newline` does `while out.ends_with([' ', '\t']) { out.pop(); }` before emitting one space; Tcl only replaces `\<nl><following-ws>` and keeps preceding spaces, so `puts "a \<nl> b"` (value `a  b`, two spaces) reformats to `puts "a b"` (value `a b`). Confidence: high

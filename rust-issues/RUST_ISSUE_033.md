# RUST_ISSUE_033: the document-sync line model counts only `\n` as a line break, but LSP mandates `\n`, `\r\n` **and lone `\r`** as EOL, so incremental edits on bare-CR files are spliced at the wrong offsets and corrupt the server's shadow buffer

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lexer/src/line_index.rs:84 (used by rust/tcl-lsp-server/src/lib.rs:8138)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/line_index.rs:84 (used by rust/tcl-lsp-server/src/lib.rs:8138) — the document-sync line model counts only `\n` as a line break, but LSP mandates `\n`, `\r\n` **and lone `\r`** as EOL, so incremental edits on bare-CR files are spliced at the wrong offsets and corrupt the server's shadow buffer.
Open a file containing an old-Mac `\r` (e.g. `"a\rb\nc"`): VS Code models 3 lines, the server 2 (`for (i, &b) ... if b == b'\n'`), so every `didChange` range after the `\r` resolves via `offset_at_utf16` to the wrong byte; all later positions (diagnostics, tokens, edits) diverge permanently until reopen. Confidence: high

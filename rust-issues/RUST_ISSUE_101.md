# RUST_ISSUE_101: `will_save_wait_until` formats with `FormatterConfig::default()` (only `max_line_length` resolved), ignoring the user's `tclLsp.formatting` settings that `formatting()` honours, and runs the formatter inline on the event loop with no panic containment

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:5164-5169` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-server/src/lib.rs:5164-5169 — `will_save_wait_until` formats with `FormatterConfig::default()` (only `max_line_length` resolved), ignoring the user's `tclLsp.formatting` settings that `formatting()` honours, and runs the formatter inline on the event loop with no panic containment.
A user with `formatting.indentSize=2`/`indentStyle=tabs` who enables `features.willSaveWaitUntil` gets every save re-indented with defaults, fighting explicit Format Document; and unlike every sibling handler there is no `spawn_blocking`, so a formatter panic here unwinds the event loop instead of returning a JSON-RPC error. Confidence: high

# RUST_ISSUE_175: cross-document span→range conversion runs `position_at_utf16` on the event loop against a document text that can be newer than the workspace-index span, and that function panics on a mid-UTF-8-sequence offset

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:2470-2472 (also 2170-2172)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-server/src/lib.rs:2470-2472 (also 2170-2172) — cross-document span→range conversion runs `position_at_utf16` on the event loop against a document text that can be newer than the workspace-index span, and that function panics on a mid-UTF-8-sequence offset.
`cross_document_definition` snapshots index spans, awaits, then reads the (possibly just-edited) live/disk text and calls `line_index.position_at_utf16(span.start(), &target_doc.text)`; a stale span landing inside a multi-byte char hits the documented panic with no `spawn_blocking` containment — unwinding the server. Confidence: medium

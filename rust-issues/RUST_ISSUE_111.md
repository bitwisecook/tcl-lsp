# RUST_ISSUE_111: the link range start omits `content_offset`, so a braced/quoted `source` path link underlines the opening delimiter

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/document_links.rs:187` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/document_links.rs:187 — the link range start omits `content_offset`, so a braced/quoted `source` path link underlines the opening delimiter.
`source {/tmp/foo.tcl}`: token span starts at `{` (content_offset=1), range covers `{/tmp/foo.tcl` instead of `/tmp/foo.tcl`. Should be `span.start() + content_offset` (cf. irules_context.rs:132). Same omission at :120 (`package require`). Confidence: high

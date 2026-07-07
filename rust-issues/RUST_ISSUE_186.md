# RUST_ISSUE_186: W112 (and W111:146, W115:345) subtract 1 from an exclusive-end range, undercovering by one UTF-16 unit

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/source_style.rs:182` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/source_style.rs:182 — W112 (and W111:146, W115:345) subtract 1 from an exclusive-end range, undercovering by one UTF-16 unit.
`set x 1 ` (one trailing space): ws_start=7, ws_end=8, end=7 → empty range [7,7), so the trailing-whitespace hint/fix covers/removes nothing; multi-space runs lose their last char. W115 also never strips a CRLF `\r`. `end_character: ws_end - 1,` Confidence: high

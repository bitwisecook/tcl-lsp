# RUST_ISSUE_004: fixed 2-byte escape slice splits a multibyte char and panics the whole semantic-tokens request

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | critical |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/semantic_tokens.rs:4597` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

rust/tcl-lsp-core/src/semantic_tokens.rs:4597 — fixed 2-byte escape slice splits a multibyte char and panics the whole semantic-tokens request.
`puts "\é"` (backslash immediately before any non-ASCII char; also `"a\你b"`, `"\€"`) yields a String token whose raw content `\é` reaches `push_escape_subtokens` (:3939). At the backslash the code slices two bytes, landing inside `é`: `let esc = &text[i..(i + 2).min(text.len())];` — `.min()` bounds length but not char boundary → panic, dropping all highlighting for the document. (Sibling push_regsub_subtokens:878 and regex scan_are_escape are ASCII-guarded; this one is not.) Confidence: high

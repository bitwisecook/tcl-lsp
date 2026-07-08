# RUST_ISSUE_116: non-braced property values are scanned only to end-of-line with no quote awareness, so a quoted value containing a literal newline corrupts subsequent property parsing

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-bigip/src/parser/helpers.rs:229` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-bigip/src/parser/helpers.rs:229 — non-braced property values are scanned only to end-of-line with no quote awareness, so a quoted value containing a literal newline corrupts subsequent property parsing.
`description "line one\nline two"` (REST-set descriptions store embedded newlines verbatim) → value ends at `\n`, and `line`/`two"` parse as a new key/value. Confidence: medium

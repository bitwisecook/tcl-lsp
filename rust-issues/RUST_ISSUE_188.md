# RUST_ISSUE_188: `parse_header_strict` tokenises with `split_whitespace`, ignoring quoting, so quoted identifiers with spaces are truncated on the typed-object path

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-bigip/src/model/gen/dispatch.rs:551` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip/src/model/gen/dispatch.rs:551 — `parse_header_strict` tokenises with `split_whitespace`, ignoring quoting, so quoted identifiers with spaces are truncated on the typed-object path.
`security bot-defense signature "/Common/Microsoft Access" { }` → `full_path = "/Common/Microsoft`, diverging from the quote-aware generic path. Confidence: high

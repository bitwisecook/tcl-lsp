# RUST_ISSUE_115: `parse_list_block` is not quote-aware, so a data-group record key quoted because it contains spaces is split into multiple bogus records

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-bigip/src/parser/helpers.rs:302` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip/src/parser/helpers.rs:302 — `parse_list_block` is not quote-aware, so a data-group record key quoted because it contains spaces is split into multiple bogus records.
`records { "Mozilla/5.0 (Windows)" { data blocked } }` → records `"Mozilla/5.0` and `(Windows)"` instead of one key. Header tokenising is quote-aware but list bodies aren't. Confidence: high

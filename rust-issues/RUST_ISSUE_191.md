# RUST_ISSUE_191: the quote-skip escape handling in `find_embedded_rules` lacks the `pos + 1 < len` bounds guard used in helpers.rs, letting `pos` overrun on input ending with `\` inside an unterminated quote (body silently becomes `""`, no panic)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-bigip/src/rule_extract.rs:116` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip/src/rule_extract.rs:116 — the quote-skip escape handling in `find_embedded_rules` lacks the `pos + 1 < len` bounds guard used in helpers.rs, letting `pos` overrun on input ending with `\` inside an unterminated quote (body silently becomes `""`, no panic). Confidence: high

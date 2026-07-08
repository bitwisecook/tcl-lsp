# RUST_ISSUE_046: `walk_switch` ignores the IR's `mode: SwitchMode` and `nocase`, applying glob semantics (`glob_to_prefix`) to every path switch and literal-exact semantics to every host/method arm

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/f5-xc/src/translator.rs:915-921, 1041-1156` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/f5-xc/src/translator.rs:915-921, 1041-1156 — `walk_switch` ignores the IR's `mode: SwitchMode` and `nocase`, applying glob semantics (`glob_to_prefix`) to every path switch and literal-exact semantics to every host/method arm.
`switch -regexp [HTTP::path] { {^/api/.*} { pool a } }` becomes prefix-match on the literal `^/api/.`; a plain (exact) `switch` arm containing `*` becomes a prefix match; `switch -glob [HTTP::host] { "*.example.com" {...} }` becomes exact host `"*.example.com"` which never matches. The `Statement::Switch { subject, arms, default_body, span, .. }` destructure discards `mode`/`nocase` that ir.rs explicitly carries.
Confidence: high

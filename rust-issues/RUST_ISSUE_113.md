# RUST_ISSUE_113: the `class match` mock ignores the comparison operator entirely, evaluating `starts_with`/`contains`/`ends_with` as exact equality

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-irule-test/tcl/command_mocks.tcl:1346` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-irule-test/tcl/command_mocks.tcl:1346 — the `class match` mock ignores the comparison operator entirely, evaluating `starts_with`/`contains`/`ends_with` as exact equality.
Both switch arms run the identical `::state::datagroup::match ...` and `_match_string` only does `string equal`. `if {[class match [HTTP::uri] starts_with uri_prefixes]}` with record `/api` and URI `/api/v1/x` returns 0 in the simulator but 1 on real TMM — wrong verdicts. Confidence: high

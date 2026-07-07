# RUST_ISSUE_047: fall-through switch arms (`"/a*" -`) are skipped without attaching their pattern to the shared body and without any diagnostic, so that pattern's traffic is silently dropped from the translation

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/f5-xc/src/translator.rs:1055-1059 (also 1080-1084, 1108-1112)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/f5-xc/src/translator.rs:1055-1059 (also 1080-1084, 1108-1112) — fall-through switch arms (`"/a*" -`) are skipped without attaching their pattern to the shared body and without any diagnostic, so that pattern's traffic is silently dropped from the translation.
`switch -glob [HTTP::path] { "/a*" - "/b*" { pool x } }` emits only a `/b`-prefix route; `/a*` requests are unrouted and the item list still reports the switch as fully `Translated` (XC101). Quote: `let Some(body) = arm.body.as_ref() else { continue; }; if arm.fallthrough { continue; }` — fallthrough arms have `body: None` per ir.rs.
Confidence: high

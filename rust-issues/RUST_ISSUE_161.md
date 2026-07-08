# RUST_ISSUE_161: `closer_position` treats a bare `\r` last-inner byte as a line break, but `LineIndex` counts only `\n` (the #537 invariant)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/ranges.rs:64-67` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/ranges.rs:64-67 — `closer_position` treats a bare `\r` last-inner byte as a line break, but `LineIndex` counts only `\n` (the #537 invariant).
For `{a\r}`, `word_end_position` reports the closer at line 1 col 0 while `position_at(3)` says line 0 col 3 — inconsistent/nonexistent position in a 1-line document. Confidence: high

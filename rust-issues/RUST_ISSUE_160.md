# RUST_ISSUE_160: `token_text`'s Esc empty-clamp fires on any 1-char ESC equal to `"`, including a *literal* mid-word quote with `content_offset == 0`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/source_map.rs:191-195` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/source_map.rs:191-195 — `token_text`'s Esc empty-clamp fires on any 1-char ESC equal to `"`, including a *literal* mid-word quote with `content_offset == 0`.
`set x $a"` — trailing `"` lexes as 1-byte ESC; `token_text` returns `""` instead of `"`; also makes `word_closer_offset` misclassify it as an empty quoted word. Should require `tok.content_offset != 0`. Confidence: high

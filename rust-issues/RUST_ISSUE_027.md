# RUST_ISSUE_027: `parse_command`'s ghost-`]` branch, when the ghost does not bring `level` to 0, advances `self.pos` past the *real* byte at that offset and never removes the ghost entry, violating the "consuming a ghost is zero-width" contract

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/lexer.rs:1064-1072` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/lexer.rs:1064-1072 — `parse_command`'s ghost-`]` branch, when the ghost does not bring `level` to 0, advances `self.pos` past the *real* byte at that offset and never removes the ghost entry, violating the "consuming a ghost is zero-width" contract.
With nesting ≥ 2 at a ghost offset the real byte is silently skipped (E201 recovery mis-boundaries after recovery); via the public `with_ghosts` API a ghost at EOF with `level ≥ 2` drives `pos` to `len+1`, producing a token span past the buffer that panics `SourceMap::text`. Confidence: high

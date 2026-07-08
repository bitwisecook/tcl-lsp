# RUST_ISSUE_088: the escape arm sets `newword = false` after *every* `\X` pair, but a backslash-newline is a word separator, so a `{`/`"` after a line continuation is not recognised as a word opener

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/structural_index.rs:314-323 (Builder::scan_top) and 756-762 (BraceBuilder::scan_script)` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/structural_index.rs:314-323 (Builder::scan_top) and 756-762 (BraceBuilder::scan_script) — the escape arm sets `newword = false` after *every* `\X` pair, but a backslash-newline is a word separator, so a `{`/`"` after a line continuation is not recognised as a word opener.
`BracketIndex::build("\\\n{[}")` reports `unterminated_count() == 1` while the lexer lexes `{[}` as a brace word with no warning — breaks the module's 8000/8000 "faithfulness" invariant and the `is_inert` veto in `detect_e201`, letting E201 fixes land inside brace words following a `\`-continued line. Confidence: high

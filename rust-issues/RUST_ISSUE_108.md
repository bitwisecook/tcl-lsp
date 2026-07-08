# RUST_ISSUE_108: `find_var_at_position`'s left-scan `stop_chars` omits `$`, so the first var of a `$a$b` concatenation is always resolved

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/hover.rs:1978` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/hover.rs:1978 — `find_var_at_position`'s left-scan `stop_chars` omits `$`, so the first var of a `$a$b` concatenation is always resolved.
`set z $x$y`, cursor on `y`: scan walks left across `$y` and `x` to the first `$`, name scan halts at next `$`, returns `"x"`. Corrupts hover AND goto-definition (definition.rs:123 shares this fn). The module's own WORD_DELIMS (:88) does include `$`. Confidence: high

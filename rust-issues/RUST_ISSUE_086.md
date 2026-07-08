# RUST_ISSUE_086: `variable()` accepts *single* colons in bare `$name` scans (`|| self.b[self.i] == b':'`), unlike real Tcl (`::` pairs only) and unlike the main lexer's `parse_var`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/expr_lexer.rs:328-334` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/expr_lexer.rs:328-334 — `variable()` accepts *single* colons in bare `$name` scans (`|| self.b[self.i] == b':'`), unlike real Tcl (`::` pairs only) and unlike the main lexer's `parse_var`.
The common spaceless ternary `expr {$x>0?$y:$z}` lexes `$y:` as one Variable token (TernaryC swallowed), so the Pratt parser fails `expect(TernaryC)` and the whole expression degrades to `ExprNode::Raw` — losing folding/analysis; `$a:b` also misreports the variable name as `a:b`. Confidence: high

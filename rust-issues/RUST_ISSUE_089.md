# RUST_ISSUE_089: `needs_parens_for_binary_child` only parenthesises `Binary` children (`else { return false }`), so a `Ternary` child of a binary renders without parens and re-parses with different semantics

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-syntax/src/expr/ast.rs:498-518` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-syntax/src/expr/ast.rs:498-518 — `needs_parens_for_binary_child` only parenthesises `Binary` children (`else { return false }`), so a `Ternary` child of a binary renders without parens and re-parses with different semantics.
`render_expr(parse_expr("($a ? 1 : 2) + 3"))` → `"$a ? 1 : 2 + 3"`, which re-parses as `$a ? 1 : (2 + 3)`; `expr_text` feeds optimiser/code-action replacement text, so a rewrite can change program meaning. Confidence: high

# RUST_ISSUE_152: `parse_return_subst` only strips the outer `[…]` and doesn't verify the value is a single balanced command substitution, so an O121 `tailcall` rewrite can be built from a malformed split

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Optimiser passes (inlining/taint/expr-simplify) |
| **Location** | `rust/tcl-compiler/src/optimiser/tail_call.rs:619` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/optimiser/tail_call.rs:619 — `parse_return_subst` only strips the outer `[…]` and doesn't verify the value is a single balanced command substitution, so an O121 `tailcall` rewrite can be built from a malformed split.
`return [a $x][b $y]` (legal concat) yields `inner = "a $x][b $y"`, head `a` ∈ self-names → replacement `tailcall a $x][b $y` (syntactically invalid) offered as a fix. Confidence: high (mechanism), low severity

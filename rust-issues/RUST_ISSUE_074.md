# RUST_ISSUE_074: additive constant-reassociation that cancels to zero emits the lone non-constant term BARE, dropping its numeric-coercion error; the multiplicative path guards this exact case but the additive path does not

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Optimiser passes (inlining/taint/expr-simplify) |
| **Location** | `rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs:512 (build_add_expr)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs:512 (`build_add_expr`) — additive constant-reassociation that cancels to zero emits the lone non-constant term BARE, dropping its numeric-coercion error; the multiplicative path guards this exact case but the additive path does not.
`$a + 5 - 5` (`terms=[$a]`, `constant=0`) returns `$a` unwrapped. `if {[catch {expr {$a + 5 - 5}} r]} {…}` with `$a` non-numeric errors originally (takes catch); after O110 rewrite to `expr {$a}` it succeeds — different control flow. Also changes double results (`$a + 1 + 2` vs `$a + 3` at `$a == 2**53`). Confidence: high

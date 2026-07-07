# RUST_ISSUE_026: `command()` and `quoted()` double-advance over a trailing backslash, pushing `i` to `len+1`, and `tok()` (line 184: `text: self.s[start..self.i]`) then slices out of bounds → panic

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-lexer/src/expr_lexer.rs:359-364 (also 373-376)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lexer/src/expr_lexer.rs:359-364 (also 373-376) — `command()` and `quoted()` double-advance over a trailing backslash, pushing `i` to `len+1`, and `tok()` (line 184: `text: self.s[start..self.i]`) then slices out of bounds → panic.
Input `[a \` (or `"a\`): at the final `\`, `self.i += 1` inside the match plus the unconditional `self.i += 1` yields `i = len+1`; the loop exits and `self.s[start..len+1]` panics. Reachable: `parse_expr`/`ExprParenIndex::build`/semantic-token highlighting call `tokenise_expr` on extracted `expr`/`if` bodies, so a file ending `if {$x && [foo \` crashes the analyser. Confidence: high

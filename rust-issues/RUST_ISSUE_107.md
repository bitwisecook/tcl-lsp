# RUST_ISSUE_107: Converting `if {$x eq $y} … elseif {$x eq $z} …` produces braced switch patterns `$y`/`$z` that are never substituted

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/refactor/if_to_switch.rs:203-213,229-232` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/refactor/if_to_switch.rs:203-213,229-232 — Converting `if {$x eq $y} … elseif {$x eq $z} …` produces braced switch patterns `$y`/`$z` that are never substituted.
`parse_eq_test` accepts any RHS value (no `$`/`[` guard), and the emitted `switch -exact -- $x {` + `{inner}$y {` puts the value inside a braced case list where Tcl treats it as the literal string `$y` — the branch can no longer match. Confidence: high

# RUST_ISSUE_181: array-element completion's replace range runs to end-of-line when the `$arr(...` reference is unclosed, so accepting the item deletes trailing text

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/completion.rs:778` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-core/src/completion.rs:778 — array-element completion's replace range runs to end-of-line when the `$arr(...` reference is unclosed, so accepting the item deletes trailing text.
`set v $arr(k more stuff`, cursor after `k`: no `)`, `arr_end` walks to EOL, accepting `$arr(key)` replaces `$…`→EOL, eating ` more stuff`. Confidence: medium

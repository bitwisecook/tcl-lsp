# RUST_ISSUE_179: `inline_proc_action` splits call args with `split_whitespace` and substitutes params by plain string `replace`, corrupting braced arguments and prefix-sharing variable names

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/code_actions.rs:1284,1325-1332` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/code_actions.rs:1284,1325-1332 — `inline_proc_action` splits call args with `split_whitespace` and substitutes params by plain string `replace`, corrupting braced arguments and prefix-sharing variable names.
`f {a b}` yields `call_args = ["{a", "b}"]` so `$p` becomes `{a`; and param `n` with body `$nn` → `.replace("$n", arg)` turns `$nn` into `<arg>n`. Confidence: high

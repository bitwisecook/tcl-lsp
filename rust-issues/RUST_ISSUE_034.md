# RUST_ISSUE_034: Variable rename rewrites the declaration span with the bare new name, dropping any array-element suffix

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/rename.rs:605-615` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/rename.rs:605-615 — Variable rename rewrites the declaration span with the bare new name, dropping any array-element suffix.
For `set arr(0) 1\nputs $arr(0)`, renaming `arr`→`data` produces `set data 1` (the `(0)` is deleted; refs become `$data(0)`, so the script now errors "variable isn't array"). The analyser's `definition_span` covers the whole `arr(0)` token (handlers.rs:144 passes `None` → `tok.span`), but the decl edit is only `match def_text.rfind("::") { ... None => new_name.to_owned() }` — no `split_array_suffix` like `build_var_ref_replacement` uses. The existing test only asserts the *reference* edit text. Confidence: high

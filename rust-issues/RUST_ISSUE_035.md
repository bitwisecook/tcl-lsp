# RUST_ISSUE_035: Proc rename rewrites call sites of *different* same-named procs in other namespaces because the invocation match lacks the namespace gate that `references.rs` applies

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/rename.rs:691-697` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/rename.rs:691-697 — Proc rename rewrites call sites of *different* same-named procs in other namespaces because the invocation match lacks the namespace gate that `references.rs` applies.
`proc ::a::helper {} {}` + `proc ::b::helper {} {}` + `namespace eval ::b { helper }`: renaming `::a::helper` rewrites the `helper` call in `::b` (breaking it), since `let matches = inv.name == proc_def.name || ...` is an unconditional OR, while `proc_reference_spans` (references.rs:109-118) requires `call_ns == target_ns` for simple-name matches. Rename and find-references disagree; rename corrupts. Same shape in `rename_class` (rename.rs:741-743). Confidence: high

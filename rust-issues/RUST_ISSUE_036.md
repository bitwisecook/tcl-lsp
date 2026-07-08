# RUST_ISSUE_036: Cross-document rename matches sibling-file definitions and calls by bare simple name, renaming unrelated same-named procs and even moving them into the wrong namespace

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/workspace_index.rs:620-627,687-697 + rename.rs:1036-1049` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/workspace_index.rs:620-627,687-697 + rename.rs:1036-1049 — Cross-document rename matches sibling-file definitions and calls by bare simple name, renaming unrelated same-named procs and even moving them into the wrong namespace.
With `::a::helper` in a.tcl and `::b::helper` in b.tcl, renaming `::a::helper`→`foo` calls `index.proc_definitions("helper", …)` which matches `p.name == name` regardless of namespace, and the decl replacement is `new_decl_text` = `::a::foo` — so `proc helper` inside `namespace eval ::b` in b.tcl is rewritten to `proc ::a::foo`. `invocations_of` (`i.name == simple_name`) over-rewrites calls the same way. Confidence: high

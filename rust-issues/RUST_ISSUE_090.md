# RUST_ISSUE_090: `dict_path_set`/`dict_path_unset` LEAK the duplicated intermediate sub-dict on a nested error path

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_dict.rs:566 (and :634)` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

runtime/rust/src/cmd_dict.rs:566 (and :634) — `dict_path_set`/`dict_path_unset` LEAK the duplicated intermediate sub-dict on a nested error path.
`sub = obj::duplicate(s)` (rc 0) for a shared intermediate, but `dict_path_set(sub, rest, value)?` returns before `dict::dict_set(dict,*head,sub)` retains it, orphaning `sub`. `set d {a x}; set e $d; dict set d a b 9` errors like Tcl but leaks the duplicate. Confidence: high

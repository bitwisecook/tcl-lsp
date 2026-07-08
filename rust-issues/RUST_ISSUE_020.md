# RUST_ISSUE_020: α-renaming does not rewrite variable references *inside* an array index, only the array base, so an inlined body captures the caller's variable

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Optimiser passes (inlining/taint/expr-simplify) |
| **Location** | `rust/tcl-compiler/src/inlining/rename.rs:74 (and :501-516)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/inlining/rename.rs:74 (and :501-516) — α-renaming does not rewrite variable references *inside* an array index, only the array base, so an inlined body captures the caller's variable.
`rename_var_name`/`rewrite_value_string` split `arr($idx)` into base `arr` + verbatim tail `($idx)`; base is renamed but `$idx` in the tail is left untouched. `proc helper {} { set arr(0) zero; set arr(1) one; set idx 1; return $arr($idx) }` inlined (tail) into `proc caller {} { set idx 0; helper }` becomes `… set __inline_arr(1) one; set __inline_idx 1; return $__inline_arr($idx)` — the index `$idx` reads the *caller's* `idx`=0, so `caller` returns `zero` not `one`. `collect_local_names` explicitly admits array writes, so reachable. Confidence: high

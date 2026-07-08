# RUST_ISSUE_122: list subtraction (`-` and `-=`) compares elements with raw `py_eq`, which never equates a `PathRef` with an equal string, so removing config references by path string silently removes nothing

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/tcl-bigip-query/src/eval.rs:1274-1287` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-bigip-query/src/eval.rs:1274-1287 — list subtraction (`-` and `-=`) compares elements with raw `py_eq`, which never equates a `PathRef` with an equal string, so removing config references by path string silently removes nothing.
`[.ltm.virtual[] | .rules] | .[0] - ["/Common/rule1"]` (rules project as PathRefs) returns the list unchanged; `bi_contains`/`bi_index` deliberately coerce PathRef→Str before `py_eq`, and `apply_scalar_binop` coerces for `==`, so the missing coercion here is an inconsistency, not a design choice. Quote: `.filter(|item| !b.iter().any(|x| value::py_eq(item, x)))`.
Confidence: medium

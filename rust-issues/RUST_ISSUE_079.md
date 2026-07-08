# RUST_ISSUE_079: `fold_split`'s default split set omits `\r`; real Tcl's `split` default is `" \n\t\r"`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/const_fold.rs:232` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-registry/src/const_fold.rs:232 — `fold_split`'s default split set omits `\r`; real Tcl's `split` default is `" \n\t\r"`.
`split "a\r\nb"` → tclsh `a {} b`, fold emits `{a\r} b` — a wrong constant fold, not a conservative bail. `[s] => (*s, " \t\n"),` Confidence: high

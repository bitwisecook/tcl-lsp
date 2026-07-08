# RUST_ISSUE_024: `leading_zero_is_octal` returns octal=true for a tcl9.1 registry because it tests only the `TCL90` bit

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/registry.rs:311` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

rust/tcl-registry/src/registry.rs:311 — `leading_zero_is_octal` returns octal=true for a tcl9.1 registry because it tests only the `TCL90` bit.
`registry_for_dialect("tcl9.1")` loads only `DialectSet::TCL91`, so a tcl9.1 document folds `expr {010 == 8}` under the 8.x octal rule, though 9.1 keeps TIP 472 decimal. Consumed at tcl-compiler/src/compilation_unit.rs:270; the compiler's parallel string-based helper (tcl_expr_eval.rs:148 `!dialect.starts_with("tcl9")`) gets 9.1 right, so the two live paths disagree. `pub fn leading_zero_is_octal(&self) -> bool { !self.loaded_dialects.contains(DialectSet::TCL90) }` Confidence: high

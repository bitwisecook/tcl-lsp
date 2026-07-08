# RUST_ISSUE_083: `TclVersion::from_dialect` has no `"tcl9.1"` arm (enum has no `V9_1`), so every versioned const-fold treats tcl9.1 as an *unknown* dialect

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/hooks.rs:215` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-registry/src/hooks.rs:215 — `TclVersion::from_dialect` has no `"tcl9.1"` arm (enum has no `V9_1`), so every versioned const-fold treats tcl9.1 as an *unknown* dialect.
Under `--dialect tcl9.1`, `run_const_fold` maps to `None` and folds like `string is integer 5000000000`, versioned `format`/`scan` degrade to the "dialect-invariant subset" where 9.1 should behave as 9.0+. Conservative but silently divergent. Confidence: high

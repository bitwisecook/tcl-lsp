# RUST_ISSUE_072: `lower_append_lappend` ignores `{*}` expansion and substituted names, recording a def of the wrong variable

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler lowering / variable scoping |
| **Location** | `rust/tcl-compiler/src/lowering_hooks.rs:294` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/lowering_hooks.rs:294 — `lower_append_lappend` ignores `{*}` expansion and substituted names, recording a def of the wrong variable.
Neither the `has_expansion(cmd)` guard that set/incr/expr/return carry, nor the arg-kind check `lower_set` uses; blindly does `defs: vec![normalise_var_name(&cmd.args[0])]`. `append {*}$args` records `defs=["args"]` (args is the list being expanded, read — not the write target); `append $x foo` records `defs=["x"]` though real Tcl writes `$x`'s target. Contrast `incr $x`, which keeps the raw name so `resolve_place` yields Unknown. SSA/def-use gets a spurious concrete def. Confidence: high

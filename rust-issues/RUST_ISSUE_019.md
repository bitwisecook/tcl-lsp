# RUST_ISSUE_019: under `-nocommands`, `[...]` is copied verbatim, so `$var`/`\` inside brackets are never substituted; Tcl only skips *command* substitution, not variable/backslash

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Compiler front-end (segmenter/expr/subst) |
| **Location** | `rust/tcl-compiler/src/subst_nocommands.rs:126` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/subst_nocommands.rs:126 — under `-nocommands`, `[...]` is copied verbatim, so `$var`/`\` inside brackets are never substituted; Tcl only skips *command* substitution, not variable/backslash.
Confirmed against the VM's own `subst_command` (subst.rs:228): with `commands=false` the `[` arm is skipped but `$`/`\` still fire. `subst -nocommands {[dict get \$obj $field]}` with `field=email` yields `[dict get $obj email]` in real Tcl but returns the template verbatim here. Result is compiled into a factory/proc body → materialised body is wrong. Secondary: `match_bracket` is brace-unaware, so `[foo {]} $bar]` mis-spans. Confidence: high

# RUST_ISSUE_151: `"default"` is in `INTERPRETER_GLOBAL_SUBCOMMANDS` ("safe"), but `info default procname arg varname` writes `varname` in the current frame

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Compiler lowering / variable scoping |
| **Location** | `rust/tcl-compiler/src/var_escape/info_subcommands.rs:44` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/var_escape/info_subcommands.rs:44 — `"default"` is in `INTERPRETER_GLOBAL_SUBCOMMANDS` ("safe"), but `info default procname arg varname` writes `varname` in the current frame.
The registry marks this a frame write (info_.rs:201 gives `default` `arg_roles: &[(2, ArgRole::VarWrite)]`) yet `handle_info` short-circuits via `is_safe_info_subcommand` and never escapes `varname`. Same class/mitigation as the `uplevel 0` gap. Confidence: medium

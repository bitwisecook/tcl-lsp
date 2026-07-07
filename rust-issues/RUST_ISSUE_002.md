# RUST_ISSUE_002: uplevel-passthrough inlining splices the body into the caller without rejecting `return`/`break`/`continue`, changing return-code semantics

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | critical |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/inline_uplevel.rs:183-227` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/inline_uplevel.rs:183-227 — uplevel-passthrough inlining splices the body into the caller without rejecting `return`/`break`/`continue`, changing return-code semantics.
`proc run {b} { uplevel 1 $b }; proc c {} { run { return 5 }; puts after }` — real Tcl: TCL_RETURN propagates through `uplevel` and is absorbed at `run`'s proc boundary, so `c` prints "after"; after `inline_uplevel_passthrough` (run unconditionally in `CompilationUnit::build_for_inner`, compilation_unit.rs:579) the spliced `return 5` returns from `c`, so downstream CFG/codegen/dead-code analysis treat `puts after` as unreachable. `body_has_frame_reach` gates only `uplevel|upvar|UpFrame` — the general inliner (`inlining/mod.rs`, `has_irreturn_in_unsafe_scope`) shows the return gate was known-needed and is absent here. Confidence: high

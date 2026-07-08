# RUST_ISSUE_002: uplevel-passthrough inlining splices the body into the caller without rejecting `return`/`break`/`continue`, changing return-code semantics

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | critical |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/inline_uplevel.rs:183-227` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/inline_uplevel.rs:183-227 — uplevel-passthrough inlining splices the body into the caller without rejecting `return`/`break`/`continue`, changing return-code semantics.
`proc run {b} { uplevel 1 $b }; proc c {} { run { return 5 }; puts after }` — real Tcl: TCL_RETURN propagates through `uplevel` and is absorbed at `run`'s proc boundary, so `c` prints "after"; after `inline_uplevel_passthrough` (run unconditionally in `CompilationUnit::build_for_inner`, compilation_unit.rs:579) the spliced `return 5` returns from `c`, so downstream CFG/codegen/dead-code analysis treat `puts after` as unreachable. `body_has_frame_reach` gates only `uplevel|upvar|UpFrame` — the general inliner (`inlining/mod.rs`, `has_irreturn_in_unsafe_scope`) shows the return gate was known-needed and is absent here. Confidence: high

## Resolution

Fixed. Both uplevel-passthrough inlining paths (`static_passthrough_body` and
the `ParamBody` callsite rewriter) now reject a body that can complete with a
`return`/`break`/`continue` escaping its own top level, via the new
`body_has_completion_escape`. The erased passthrough proc boundary is what
decremented a `return`'s level and turned a raw `break`/`continue` into an
`invoked "…" outside of a loop` error; splicing directly would leak those codes
into the caller. Loops absorb `break`/`continue` in their own body (but not
`return`); `catch` absorbs every non-`OK` code, so neither contributes an
escape. Regression tests: `static_passthrough_with_{return,bare_break,
bare_continue}_rejected`, `static_passthrough_with_{loop_absorbed_break,
catch_absorbed_return}_allowed`, `static_passthrough_return_inside_loop_still_
rejected`, and `param_body_passthrough_{return_callsite_not_inlined,
plain_callsite_still_inlined}` in `inline_uplevel.rs`.

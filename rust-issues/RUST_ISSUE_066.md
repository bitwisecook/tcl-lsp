# RUST_ISSUE_066: type propagation's phi join has the same version-0 skip (`if ver == 0 { continue; }`), so a conditionally-assigned parameter is typed solely from the assigned arm

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/type_infer.rs:732-734` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/type_infer.rs:732-734 — type propagation's phi join has the same version-0 skip (`if ver == 0 { continue; }`), so a conditionally-assigned parameter is typed solely from the assigned arm.
`proc p {c x} { if {$c} { set x 5 }; …use $x… }` — the merge phi types `x` as Known Int even though the caller may pass any string; consumers (S101 shimmer hints, the W307/W308 type-mismatch emitters, `infer_function_return_type`) report facts that are false for the not-taken path. Wrong-diagnostic class, no rewrite. Confidence: high

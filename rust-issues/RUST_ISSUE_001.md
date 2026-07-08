# RUST_ISSUE_001: SCCP's phi join skips version-0 incoming operands, so a parameter's (or any live-in's) runtime value silently vanishes from the merge and the phi folds to the defined-arm constant

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | critical |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/sccp.rs:455-457` |
| **Status** | Fixed |
| **Verification** | Verified firsthand by reviewer |

## Finding

rust/tcl-compiler/src/sccp.rs:455-457 — SCCP's phi join skips version-0 incoming operands, so a parameter's (or any live-in's) runtime value silently vanishes from the merge and the phi folds to the defined-arm constant.
`proc p {c x} { if {$c} { set x 5 }; if {$x == 5} {A} else {B} }` — phi for `x` at the merge has incoming `{entry:0, then:1}`; `if incoming_ver == 0 { continue; }` drops the caller's `x`, phi = `Const(5)`, the second branch folds always-true: else-arm becomes SCCP-unreachable (O107 "Eliminate unreachable dead code" deletes live code, branch_folding emits O101 rewriting `{$x == 5}`→`{1}`, `sccp_constants_for` propagates `x=5` elsewhere). `seed_live_in_roots`'s own doc says live-ins are seeded Overdefined precisely so they don't "silently vanish from any phi", but the seeder also excludes v0 phi feeds (`if *inc > 0`) and the phi loop never looks v0 up. The sibling interval pass does it right (`intervals.rs:569-574` joins TOP for `inc == 0`), confirming intent. Confidence: high

## Resolution

Fixed. `sccp_process_phis` no longer skips a version-0 incoming operand: it
joins the live-in value in (defaulting to `Overdefined` when unseeded), so a
phi merging a live-in with a defined-arm constant correctly widens to
`Overdefined` — mirroring the interval pass, which joins `TOP` for `inc == 0`.
`seed_live_in_roots` now also records version-0 phi feeds so `(name, 0)` is
pinned `Overdefined` in the lattice. Regression tests:
`phi_join_widens_version_zero_livein_to_overdefined` and
`phi_join_defaults_missing_version_zero_feed_to_overdefined` in `sccp.rs`.

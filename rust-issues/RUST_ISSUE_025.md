# RUST_ISSUE_025: `interp issafe`/`exists`/`hidden` declare `Arity::exact(1)` but the path argument is optional in every Tcl release (`interp issafe ?path?`; `interp exists` with no path returns 1)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/commands/tcl/interp.rs:213` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

rust/tcl-registry/src/commands/tcl/interp.rs:213 — `interp issafe`/`exists`/`hidden` declare `Arity::exact(1)` but the path argument is optional in every Tcl release (`interp issafe ?path?`; `interp exists` with no path returns 1).
The per-subcommand arity check (PR #803) flags the standard zero-arg idiom `interp issafe` as wrong-#-args in all dialects. Same defect at interp.rs:145 (`exists`) and :162 (`hidden`). Also interp.rs:92 `create` is `Arity::new(0,2)` while the legal `interp create -safe -- name` is 3 words. Confidence: high

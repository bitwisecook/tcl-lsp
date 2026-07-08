# RUST_ISSUE_157: TclOO's `private` is modelled as a flat single-body member, but oo::define's `private` also takes the wrapper form `private method m {} {…}`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/definer.rs:196` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-registry/src/definer.rs:196 — TclOO's `private` is modelled as a flat single-body member, but oo::define's `private` also takes the wrapper form `private method m {} {…}`.
The grammar marks arg 0 as `Body`, so a walker treats the literal word `method` as a script and never applies METHOD_ROLES to the wrapped definition; itcl's identical-shaped `public`/`protected`/`private` are correctly `wrapper`. Confidence: medium

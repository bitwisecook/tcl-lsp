# RUST_ISSUE_158: `resolve_option_terminator` matches the subcommand word exactly, so a legal prefix abbreviation loses the subcommand-scoped `--` terminator profile (missed W304). Inconsistent with `arg_indices_for_role`'s `resolve_subcommand` after PR #803

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/registry.rs:907` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-registry/src/registry.rs:907 — `resolve_option_terminator` matches the subcommand word exactly, so a legal prefix abbreviation loses the subcommand-scoped `--` terminator profile (missed W304). Inconsistent with `arg_indices_for_role`'s `resolve_subcommand` after PR #803. Confidence: high

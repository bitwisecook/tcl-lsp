# RUST_ISSUE_197: `tcl pkg why` finds dependents by substring match on `"name@version"` strings, producing false "required by" entries

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/pkg.rs:877` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/pkg.rs:877 — `tcl pkg why` finds dependents by substring match on `"name@version"` strings, producing false "required by" entries.
`tcl pkg why http` lists a package whose requires contain `shttp@1.0` or `http2@…` as a dependent: `other.requires.iter().any(|r| r.contains(package))` instead of comparing the name before `@`. Confidence: high

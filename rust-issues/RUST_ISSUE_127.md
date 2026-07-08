# RUST_ISSUE_127: `tcl pkg verify` ("Verify integrity hashes") never recomputes or compares any hash; it only checks the lockfile's integrity string is non-empty

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/pkg.rs:546` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/pkg.rs:546 — `tcl pkg verify` ("Verify integrity hashes") never recomputes or compares any hash; it only checks the lockfile's integrity string is non-empty.
Corrupt or tamper any file under `lib/<pkg>-<ver>/` (or the CAS tree): `pkg verify` still prints `✓` per package and exits 0 — the loop is just `if pkg.integrity.is_empty()`; `cas::verify_integrity` exists but is never called. Confidence: high

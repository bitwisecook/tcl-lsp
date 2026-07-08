# RUST_ISSUE_049: the `deny_network` policy floor ("Force the network off regardless of what a profile requests") enforces nothing: execution proceeds with full network access

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-sandbox/src/lib.rs:489` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-sandbox/src/lib.rs:489 — the `deny_network` policy floor ("Force the network off regardless of what a profile requests") enforces nothing: execution proceeds with full network access.
An operator sets `deny-network = true` in pkg-policy.toml (without `fail-closed`); `tcl pkg build`/hooks/`git` children still have unrestricted network — `run()` only flips the flags `want_network`/`network_enforced` and Baseline's `configure()` is a no-op, so no restriction is ever applied and no error is raised. `let want_network = profile.allow_network && !policy.deny_network; let network_enforced = want_network || confinement.enforces_network_deny();` — the only guard is the `require_network_deny && fail_closed` combination. `tcl pkg policy show` still displays `deny-network true` as effective. Confidence: high

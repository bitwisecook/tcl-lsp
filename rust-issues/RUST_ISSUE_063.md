# RUST_ISSUE_063: loose `when`-header and `profile{}`-body parsing silently ignore malformed/extra content; a verdict-less handler silently defaults to drop

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `eBPF` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

eBPF — loose `when`-header and `profile{}`-body parsing silently ignore malformed/extra content; a verdict-less handler silently defaults to drop.
`when E priority abc {…}` → non-integer priority silently defaults to 500; `when E foo bar {…}` → extra words ignored (frontend.rs:162-176). Non-`field` statements inside profile{} dropped with no error (profile.rs:186-216). Empty/verdict-less `when SOCKET_FILTER {}` compiles to synthesized "drop" with no warning (lower.rs:674-699). Confidence: high

# RUST_ISSUE_112: `simulate_irule` reads the wrong Tcl variable for the response-committed flag, so `SimOutcome::response_committed` is always `false`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-irule-test/src/sim.rs:172` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-irule-test/src/sim.rs:172 — `simulate_irule` reads the wrong Tcl variable for the response-committed flag, so `SimOutcome::response_committed` is always `false`.
It evals `set ::state::http::response::response_committed`, but the variable is in the `http` namespace (`set ::state::http::response_committed 1`). The eval errors and `.is_ok_and(...)` swallows it, so an iRule doing `HTTP::respond`/`HTTP::redirect` is reported as not having committed a response. Confidence: high

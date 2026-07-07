# RUST_ISSUE_190: `infer_profile_type` matches SSL by substring but HTTP/TCP/UDP only by exact name, so ordinary custom profile names never map and the generated orchestrator omits the HTTP profile

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-irule-test/src/topology.rs:344` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-irule-test/src/topology.rs:344 — `infer_profile_type` matches SSL by substring but HTTP/TCP/UDP only by exact name, so ordinary custom profile names never map and the generated orchestrator omits the HTTP profile.
`/Common/my_http_profile` → `name == "http"` fails, so `when HTTP_REQUEST` handlers never fire in generated tests. Confidence: medium

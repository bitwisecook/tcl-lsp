# RUST_ISSUE_048: `cn()` slices `p[..3]` without a char-boundary check, panicking on any DN component whose 3rd byte falls inside a multi-byte character

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/bigip-report-gen/rust/src/certs.rs:96` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/bigip-report-gen/rust/src/certs.rs:96 — `cn()` slices `p[..3]` without a char-boundary check, panicking on any DN component whose 3rd byte falls inside a multi-byte character.
A certificate subject/issuer like `CN=x, O=émission` (component `O=é…`: `é` occupies bytes 2-3) makes `p[..3]` panic ("byte index 3 is not a char boundary"), crashing native report generation and aborting the wasm generator; subjects come from attacker-suppliable PEMs in a UCS and from config `subject "..."` metadata. Quote: `if p.len() >= 3 && p[..3].eq_ignore_ascii_case("CN=")`.
Confidence: high

# RUST_ISSUE_138: stated Rust minimum "1.95+" contradicts the enforced MSRV `rust-version = "1.96"` (Cargo.toml:128); README.md:2066 already says 1.96. [VERIFIED — same as gate finding G4. User confirmed 1.96+ correct.]

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `Makefile:13 + error strings (532,547,559,571,582,600,729,760) and AGENTS.md:107,164` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

Makefile:13 + error strings (532,547,559,571,582,600,729,760) and AGENTS.md:107,164 — stated Rust minimum "1.95+" contradicts the enforced MSRV `rust-version = "1.96"` (Cargo.toml:128); README.md:2066 already says 1.96. [VERIFIED — same as gate finding G4. User confirmed 1.96+ correct.]
Neither cited "source of truth" (rust-toolchain.toml is just `channel = "stable"`) actually holds the MSRV. Confidence: high

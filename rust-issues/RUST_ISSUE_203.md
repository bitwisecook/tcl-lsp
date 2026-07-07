# RUST_ISSUE_203: the "downstream integration" step copies the logo/favicon into retired `tooling/explorer/static` behind a `[ -d ]` guard, so `make logo` silently stops propagating the favicon to the compiler-explorer GUI (now at `rust/tcl-cli/gui`)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/build/render_logo.sh:127,137` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

scripts/build/render_logo.sh:127,137 — the "downstream integration" step copies the logo/favicon into retired `tooling/explorer/static` behind a `[ -d ]` guard, so `make logo` silently stops propagating the favicon to the compiler-explorer GUI (now at `rust/tcl-cli/gui`). Confidence: high

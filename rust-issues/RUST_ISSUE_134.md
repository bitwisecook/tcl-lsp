# RUST_ISSUE_134: xtask docs-index drift gate fails (missing design-index link)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `cargo xtask kcs-index-links` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

`cargo xtask kcs-index-links` (part of `make rust-check` / `xtask-check` / CI pr-gate) fails: "design index missing link to docs/design/tcloo-object-typing.md". The other six xtask drift gates (diag-tables, editor catalogs, editor settings, vscode package, jetbrains catalog, ai diagnostics) pass.

Confidence: high (verified firsthand)

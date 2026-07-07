# RUST_ISSUE_055: `cargo fmt --all --check` fails at the branch tip

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Build tooling & CI |
| **Location** | `workspace: cargo fmt --all --check` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

The branch tip does not satisfy its own formatter gate: `cargo fmt --all --check` reports ~2026 diff hunks across ~145 files (tcl-compiler, tcl-lsp-core, tcl-registry ~90 files, bigip-report-gen, xtask/gen_vscode_package.rs, ...). The drift is import-ordering: the committed order is case-insensitive-alphabetical while rustfmt (edition/style 2024, no rustfmt.toml) wants version-sort. The GitHub Actions `pr-gate` job runs `cargo fmt --all --check` on PRs, so this would bounce every PR. Reproduced under both rustfmt 1.94.1 and 1.96.1.

Confidence: high (verified firsthand)

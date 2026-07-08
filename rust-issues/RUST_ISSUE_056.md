# RUST_ISSUE_056: `make check-rust` clippy gate fails with 12 lint sites

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Build tooling & CI |
| **Location** | `workspace: cargo clippy --workspace --all-targets -- -D warnings` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

The `make check-rust` / CI `pr-gate` clippy invocation (`cargo clippy --workspace --all-targets -- -D warnings`) exits 101. Sites: tcl-registry/src/commands/tcllib/mod.rs:307 (collapsible_if); tcl-lsp-core/src/semantic_tokens.rs:3530 (too_many_arguments), :5304/:5370 (map_unwrap_or), :6367 (too_many_lines); tcl-lsp-server/src/lib.rs:4298 (too_many_lines), :7757 (field_reassign_with_default); bigip-report-gen/rust/build.rs:29 (map_unwrap_or); bigip-report-gen/rust/src/markdown.rs:36 (must_use_candidate); tcl-bigip-query/src/architecture.rs:345/:632 (too_many_lines); tcl-cli/src/commands/transform.rs:61 (field_reassign_with_default). AGENTS.md forbids silencing these with `#[allow]`.

Confidence: high (verified firsthand)

# RUST_ISSUE_059: independent installers are chained bare under `set -e`, so any failure in `install_wasmtime`/`install_binaryen`/`install_wasi_sdk` (3 GitHub downloads, first in order) aborts before `install_rust`, leaving a pre-baked stale toolchain — matches the observed rustc 1.94.1 vs required 1.96. [Main agent independently observed 1.94.1 at session start.]

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Build tooling & CI |
| **Location** | `.claude/hooks/session-start.sh:559-567` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.claude/hooks/session-start.sh:559-567 — independent installers are chained bare under `set -e`, so any failure in `install_wasmtime`/`install_binaryen`/`install_wasi_sdk` (3 GitHub downloads, first in order) aborts before `install_rust`, leaving a pre-baked stale toolchain — matches the observed rustc 1.94.1 vs required 1.96. [Main agent independently observed 1.94.1 at session start.]
`install_wasmtime; install_binaryen; install_wasi_sdk; install_rust; ...` no `|| true` isolation, so `rustup toolchain install stable` never executes after any earlier network hiccup. Confidence: high (mechanism), medium (exact cause of 1.94.1)

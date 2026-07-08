# RUST_ISSUE_140: `SUDO` is only set in the Linux branch, so on macOS (a claimed supported platform) `ensure_wasi_sdk` runs `mkdir -p /opt/wasi-sdk-25.0` as the invoking user, failing with EPERM on root-owned /opt and aborting the whole installer under `set -e`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/dev/ensure-test-deps.sh:100-119,750-754` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

scripts/dev/ensure-test-deps.sh:100-119,750-754 — `SUDO` is only set in the Linux branch, so on macOS (a claimed supported platform) `ensure_wasi_sdk` runs `mkdir -p /opt/wasi-sdk-25.0` as the invoking user, failing with EPERM on root-owned /opt and aborting the whole installer under `set -e`.
Darwin path never assigns SUDO; `$SUDO mkdir -p "$prefix"` expands to plain `mkdir`. Confidence: high

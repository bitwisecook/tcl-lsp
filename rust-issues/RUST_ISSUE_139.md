# RUST_ISSUE_139: every scoped `ensure-*-deps` target omits `SKIP_WASI_SDK`, `SKIP_PYTHON_TK`, `SKIP_UV`, so e.g. `ensure-vscode-test-deps` ("Install xvfb") also downloads ~100 MB wasi-sdk, installs python3-tk (retired pytest), and installs uv; the omission also defeats ensure-test-deps.sh's `all_skipped` early-exit, making these targets `exit 2` on unsupported platforms even when nothing is needed

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `Makefile:831-909` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

Makefile:831-909 — every scoped `ensure-*-deps` target omits `SKIP_WASI_SDK`, `SKIP_PYTHON_TK`, `SKIP_UV`, so e.g. `ensure-vscode-test-deps` ("Install xvfb") also downloads ~100 MB wasi-sdk, installs python3-tk (retired pytest), and installs uv; the omission also defeats ensure-test-deps.sh's `all_skipped` early-exit, making these targets `exit 2` on unsupported platforms even when nothing is needed. Confidence: high

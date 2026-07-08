# RUST_ISSUE_058: `setup_python_venv` runs `uv sync --extra dev` at the repo root, but the branch retired Python (no pyproject.toml/uv.lock), so the hook fails every session and skips `setup_tcl_library`. [VERIFIED: no pyproject.toml/uv.lock.]

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Build tooling & CI |
| **Location** | `.claude/hooks/session-start.sh:491-527` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.claude/hooks/session-start.sh:491-527 — `setup_python_venv` runs `uv sync --extra dev` at the repo root, but the branch retired Python (no pyproject.toml/uv.lock), so the hook fails every session and skips `setup_tcl_library`. [VERIFIED: no pyproject.toml/uv.lock.]
`( cd "$REPO_ROOT" && uv sync --extra dev )` aborts under `set -euo pipefail`; `setup_tcl_library` (exports `TCL_LIBRARY` for the --disable-shared tclsh9.0) never runs. Confidence: high

# RUST_ISSUE_136: the bytecode/test capture tooling targets a retired top-level `tests/` tree (`tests/bytecode_snippets`, `tests/bytecode_reference`, `tests/test_reference` exist nowhere; git ls-files → 0), so `capture-bytecode-refs` (test-slow serial phase 1) is a silent no-op that mkdir-pollutes an untracked `tests/` tree

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `Makefile:911-929 + scripts/capture/bytecode.sh:39 / scripts/capture/test_results.sh:43` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

Makefile:911-929 + scripts/capture/bytecode.sh:39 / scripts/capture/test_results.sh:43 — the bytecode/test capture tooling targets a retired top-level `tests/` tree (`tests/bytecode_snippets`, `tests/bytecode_reference`, `tests/test_reference` exist nowhere; git ls-files → 0), so `capture-bytecode-refs` (test-slow serial phase 1) is a silent no-op that mkdir-pollutes an untracked `tests/` tree.
With tclsh9.0 present it iterates the unmatched glob (`base="*"`), records "0 captured, 1 failed", and still exits 0 (the `failed` counter never affects the exit code). Confidence: high

## Resolution

Fixed. The `capture-bytecode-refs` gate target — and its slot in the `test-slow`
serial batch — were removed from the `Makefile`. The top-level `tests/`
reference tree it scanned is a retired-Python leftover; bytecode/test reference
capture is now an on-demand operation via the manual `scripts/capture/*.sh`
tools (driven by the `bytecode-compare` / `test-results` skills against `tmp/`),
not a silent no-op phase inside the local gate. `make test-slow` no longer
`mkdir`-pollutes an untracked `tests/` tree.

# RUST_ISSUE_037: The trailing-whitespace pass trims *every* output line, including lines inside multi-line braced/quoted string literals, changing string data

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/formatting/engine.rs:1117-1123 (and mod.rs:215-223)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/formatting/engine.rs:1117-1123 (and mod.rs:215-223) — The trailing-whitespace pass trims *every* output line, including lines inside multi-line braced/quoted string literals, changing string data.
`set x {line1   \nline2}` formats to `set x {line1\nline2}` — `result.split('\n').map(str::trim_end)` has no awareness of string interiors, so `$x`'s value silently loses its trailing spaces (same in `finalise_slice` for range formatting). Violates "formatter never changes semantics". Confidence: high

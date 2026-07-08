# RUST_ISSUE_130: `dedent()` computes the common indent in bytes and slices `ln[common..]`, panicking on a non-char boundary when indentation mixes multibyte Unicode whitespace with ASCII spaces

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/minimize.rs:263` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-cli/src/commands/minimize.rs:263 — `dedent()` computes the common indent in bytes and slices `ln[common..]`, panicking on a non-char boundary when indentation mixes multibyte Unicode whitespace with ASCII spaces.
`tcl minimize file.tcl CODE` on a reduced snippet containing one line indented with U+00A0 (2 bytes, `char::is_whitespace` = true) and another with a single space → `common = 1` → `byte index 1 is not a char boundary` panic instead of a diagnostic. `.map(|ln| ln.len() - ln.trim_start().len())` then `ln[common..]`. Confidence: medium
